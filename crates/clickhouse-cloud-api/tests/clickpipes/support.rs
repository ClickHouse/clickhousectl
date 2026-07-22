// ClickPipes-specific AWS test infrastructure: S3/IAM/EC2/Kinesis provisioning
// helpers, Redpanda TLS cert generation, and the `AwsCleanupRegistry` that
// tracks resources for end-of-run teardown.
//
// Generic test infra (TestContext, FailureRecorder, CleanupRegistry,
// poll_until, ClickHouse provisioning, etc.) lives in `tests/common/support.rs`
// and is re-exported below so callers can pull both surfaces from a single
// `use crate::support::*;`.

#![allow(dead_code)]

pub use crate::common::support::*;

use std::time::Duration;

// ── AWS Cleanup Registry ─────────────────────────────────────────────
//
// Tracks AWS-side resources (S3 buckets, IAM roles) created by E2E tests
// against integration sources, so a single teardown call removes them
// regardless of which test step failed.

#[derive(Default)]
pub struct AwsCleanupRegistry {
    s3_buckets: Vec<(aws_sdk_s3::config::Region, String)>,
    iam_roles: Vec<String>,
    ec2_instances: Vec<String>,
    ec2_security_groups: Vec<String>,
    ec2_elastic_ips: Vec<String>,
    /// `(region, stream_name)` — the region is required so cleanup can build a
    /// regional Kinesis client even if the parent SDK config lives elsewhere.
    kinesis_streams: Vec<(String, String)>,
}

impl AwsCleanupRegistry {
    pub fn register_s3_bucket(
        &mut self,
        region: aws_sdk_s3::config::Region,
        bucket: impl Into<String>,
    ) {
        self.s3_buckets.push((region, bucket.into()));
    }

    pub fn register_iam_role(&mut self, role_name: impl Into<String>) {
        self.iam_roles.push(role_name.into());
    }

    pub fn register_ec2_instance(&mut self, instance_id: impl Into<String>) {
        self.ec2_instances.push(instance_id.into());
    }

    pub fn register_ec2_security_group(&mut self, sg_id: impl Into<String>) {
        self.ec2_security_groups.push(sg_id.into());
    }

    pub fn register_ec2_elastic_ip(&mut self, allocation_id: impl Into<String>) {
        self.ec2_elastic_ips.push(allocation_id.into());
    }

    pub fn register_kinesis_stream(
        &mut self,
        region: impl Into<String>,
        stream_name: impl Into<String>,
    ) {
        self.kinesis_streams
            .push((region.into(), stream_name.into()));
    }

    pub fn merge_from(&mut self, mut other: AwsCleanupRegistry) {
        self.s3_buckets.append(&mut other.s3_buckets);
        self.iam_roles.append(&mut other.iam_roles);
        self.ec2_instances.append(&mut other.ec2_instances);
        self.ec2_security_groups
            .append(&mut other.ec2_security_groups);
        self.ec2_elastic_ips.append(&mut other.ec2_elastic_ips);
        self.kinesis_streams.append(&mut other.kinesis_streams);
    }

    pub async fn cleanup(
        &mut self,
        aws_config: &aws_config::SdkConfig,
        iam_client: &aws_sdk_iam::Client,
        ec2_client: &aws_sdk_ec2::Client,
    ) -> Result<(), String> {
        let mut failures = Vec::new();

        while let Some((region, bucket)) = self.s3_buckets.pop() {
            // S3 calls must hit the bucket's region — re-build the client per bucket.
            let s3_config = aws_sdk_s3::config::Builder::from(aws_config)
                .region(region)
                .build();
            let s3 = aws_sdk_s3::Client::from_conf(s3_config);
            if let Err(error) = empty_and_delete_bucket(&s3, &bucket).await {
                failures.push(format!("s3 bucket {bucket}: {error}"));
            }
        }

        while let Some(role) = self.iam_roles.pop() {
            if let Err(error) = delete_iam_role(iam_client, &role).await {
                failures.push(format!("iam role {role}: {error}"));
            }
        }

        // Terminate instances first so they release their ENIs, then drop SGs.
        if !self.ec2_instances.is_empty() {
            let ids: Vec<String> = self.ec2_instances.drain(..).collect();
            if let Err(error) = terminate_and_wait(ec2_client, &ids).await {
                failures.push(format!("ec2 instances {ids:?}: {error}"));
            }
        }

        while let Some(sg_id) = self.ec2_security_groups.pop() {
            if let Err(error) = ec2_client
                .delete_security_group()
                .group_id(&sg_id)
                .send()
                .await
            {
                let msg = error.to_string();
                if !msg.contains("InvalidGroup.NotFound") {
                    failures.push(format!("ec2 sg {sg_id}: {msg}"));
                }
            }
        }

        // Elastic IPs are auto-disassociated when their instance is terminated,
        // so by this point release is unconditional.
        while let Some(allocation_id) = self.ec2_elastic_ips.pop() {
            if let Err(error) = ec2_client
                .release_address()
                .allocation_id(&allocation_id)
                .send()
                .await
            {
                let msg = error.to_string();
                if !msg.contains("InvalidAllocationID.NotFound") {
                    failures.push(format!("ec2 eip {allocation_id}: {msg}"));
                }
            }
        }

        // Kinesis streams: build a regional client per entry so cleanup works
        // even if the parent aws_config is in a different region.
        while let Some((region, stream_name)) = self.kinesis_streams.pop() {
            let kinesis_config = aws_sdk_kinesis::config::Builder::from(aws_config)
                .region(aws_sdk_kinesis::config::Region::new(region.clone()))
                .build();
            let kinesis = aws_sdk_kinesis::Client::from_conf(kinesis_config);
            if let Err(error) = delete_kinesis_stream(&kinesis, &stream_name).await {
                failures.push(format!("kinesis stream {stream_name}: {error}"));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("\n"))
        }
    }
}

async fn empty_and_delete_bucket(s3: &aws_sdk_s3::Client, bucket: &str) -> TestResult<()> {
    eprintln!("  cleanup: emptying s3 bucket");

    let mut continuation: Option<String> = None;
    loop {
        let mut req = s3.list_objects_v2().bucket(bucket);
        if let Some(token) = &continuation {
            req = req.continuation_token(token);
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) if e.to_string().contains("NoSuchBucket") => return Ok(()),
            Err(e) => return Err(e.into()),
        };

        let keys: Vec<aws_sdk_s3::types::ObjectIdentifier> = resp
            .contents()
            .iter()
            .filter_map(|o| {
                o.key().and_then(|k| {
                    aws_sdk_s3::types::ObjectIdentifier::builder()
                        .key(k)
                        .build()
                        .ok()
                })
            })
            .collect();

        if !keys.is_empty() {
            let delete = aws_sdk_s3::types::Delete::builder()
                .set_objects(Some(keys))
                .quiet(true)
                .build()?;
            s3.delete_objects()
                .bucket(bucket)
                .delete(delete)
                .send()
                .await?;
        }

        if resp.is_truncated().unwrap_or(false) {
            continuation = resp.next_continuation_token().map(|s| s.to_string());
        } else {
            break;
        }
    }

    eprintln!("  cleanup: deleting s3 bucket");
    match s3.delete_bucket().bucket(bucket).send().await {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("NoSuchBucket") => Ok(()),
        Err(e) => Err(e.into()),
    }
}

async fn delete_iam_role(iam: &aws_sdk_iam::Client, role_name: &str) -> TestResult<()> {
    eprintln!("  cleanup: detaching inline policies on iam role");

    // Inline policies first.
    let policies = match iam.list_role_policies().role_name(role_name).send().await {
        Ok(r) => r.policy_names,
        Err(e) if e.to_string().contains("NoSuchEntity") => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for name in policies {
        let _ = iam
            .delete_role_policy()
            .role_name(role_name)
            .policy_name(&name)
            .send()
            .await;
    }

    // Managed policy attachments — none expected for our tests, but be defensive.
    if let Ok(resp) = iam
        .list_attached_role_policies()
        .role_name(role_name)
        .send()
        .await
    {
        for p in resp.attached_policies() {
            if let Some(arn) = p.policy_arn() {
                let _ = iam
                    .detach_role_policy()
                    .role_name(role_name)
                    .policy_arn(arn)
                    .send()
                    .await;
            }
        }
    }

    eprintln!("  cleanup: deleting iam role");
    match iam.delete_role().role_name(role_name).send().await {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("NoSuchEntity") => Ok(()),
        Err(e) => Err(e.into()),
    }
}

// ── AWS Provisioning Helpers ─────────────────────────────────────────

/// Create a private S3 bucket with the given name in the given region, blocked
/// from public access, with `BucketOwnerEnforced` ACL semantics.
pub async fn create_private_bucket(
    s3: &aws_sdk_s3::Client,
    region: &str,
    bucket: &str,
    tags: &[(String, String)],
) -> TestResult<()> {
    use aws_sdk_s3::types::{
        BucketCannedAcl, BucketLocationConstraint, CreateBucketConfiguration, ObjectOwnership,
        OwnershipControls, OwnershipControlsRule, PublicAccessBlockConfiguration, Tag, Tagging,
    };

    let mut req = s3
        .create_bucket()
        .bucket(bucket)
        .acl(BucketCannedAcl::Private);
    // us-east-1 must NOT have a LocationConstraint; every other region must.
    if region != "us-east-1" {
        let cfg = CreateBucketConfiguration::builder()
            .location_constraint(BucketLocationConstraint::from(region))
            .build();
        req = req.create_bucket_configuration(cfg);
    }
    req.send().await?;

    s3.put_public_access_block()
        .bucket(bucket)
        .public_access_block_configuration(
            PublicAccessBlockConfiguration::builder()
                .block_public_acls(true)
                .ignore_public_acls(true)
                .block_public_policy(true)
                .restrict_public_buckets(true)
                .build(),
        )
        .send()
        .await?;

    s3.put_bucket_ownership_controls()
        .bucket(bucket)
        .ownership_controls(
            OwnershipControls::builder()
                .rules(
                    OwnershipControlsRule::builder()
                        .object_ownership(ObjectOwnership::BucketOwnerEnforced)
                        .build()?,
                )
                .build()?,
        )
        .send()
        .await?;

    if !tags.is_empty() {
        let aws_tags: Vec<Tag> = tags
            .iter()
            .map(|(k, v)| Tag::builder().key(k).value(v).build())
            .collect::<Result<_, _>>()?;
        s3.put_bucket_tagging()
            .bucket(bucket)
            .tagging(Tagging::builder().set_tag_set(Some(aws_tags)).build()?)
            .send()
            .await?;
    }

    Ok(())
}

pub async fn put_object_bytes(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    body: Vec<u8>,
    content_type: &str,
) -> TestResult<()> {
    s3.put_object()
        .bucket(bucket)
        .key(key)
        .body(body.into())
        .content_type(content_type)
        .send()
        .await?;
    Ok(())
}

// ── TLS Cert Generation (rcgen) ──────────────────────────────────────
//
// Self-signed CA + server cert (IP SAN) + client cert/key bundle, used by
// the TLS Kafka stages. All PEM-encoded.

pub struct RedpandaCerts {
    pub ca_pem: String,
    pub server_cert_pem: String,
    pub server_key_pem: String,
    pub client_cert_pem: String,
    pub client_key_pem: String,
    /// The CN we put on the client cert — Redpanda derives the mTLS user
    /// identity from this string, so ACLs must be granted to `User:{client_cn}`.
    pub client_cn: String,
}

pub fn generate_redpanda_certs(broker_ip: &str, client_cn: &str) -> TestResult<RedpandaCerts> {
    generate_redpanda_certs_with_dns_sans(broker_ip, client_cn, &[])
}

/// Same as `generate_redpanda_certs` but allows attaching extra DNS SANs to
/// the server cert. Used by the Postgres stage to exercise `--tls-host`:
/// ClickPipes connects to the IP but validates the cert against a DNS name.
pub fn generate_redpanda_certs_with_dns_sans(
    broker_ip: &str,
    client_cn: &str,
    extra_dns_sans: &[&str],
) -> TestResult<RedpandaCerts> {
    use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, SanType};

    let parsed_ip: std::net::IpAddr = broker_ip
        .parse()
        .map_err(|e| format!("invalid broker ip {broker_ip}: {e}"))?;

    // CA — used to sign both the server cert (broker presents) and the client
    // cert (ClickPipes presents for mTLS).
    let ca_key = KeyPair::generate()?;
    let mut ca_params = CertificateParams::new(Vec::<String>::new())?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "clickhousectl-e2e-test-ca");
    let ca_cert = ca_params.clone().self_signed(&ca_key)?;
    let ca_pem = ca_cert.pem();
    let issuer = Issuer::new(ca_params, ca_key);

    // Server cert: SAN = broker_ip + any caller-supplied DNS names.
    let server_key = KeyPair::generate()?;
    let mut server_params = CertificateParams::new(Vec::<String>::new())?;
    server_params
        .subject_alt_names
        .push(SanType::IpAddress(parsed_ip));
    for dns in extra_dns_sans {
        // `SanType::DnsName` wraps a private `Ia5String`; rcgen exposes
        // `TryFrom<&str>` so we go through that without naming the type.
        let ia5 = (*dns)
            .try_into()
            .map_err(|e| format!("invalid DNS SAN {dns}: {e}"))?;
        server_params.subject_alt_names.push(SanType::DnsName(ia5));
    }
    server_params
        .distinguished_name
        .push(DnType::CommonName, "redpanda-broker");
    let server_cert = server_params.signed_by(&server_key, &issuer)?;

    // Client cert: identity comes from CN.
    let client_key = KeyPair::generate()?;
    let mut client_params = CertificateParams::new(Vec::<String>::new())?;
    client_params
        .distinguished_name
        .push(DnType::CommonName, client_cn);
    let client_cert = client_params.signed_by(&client_key, &issuer)?;

    Ok(RedpandaCerts {
        ca_pem,
        server_cert_pem: server_cert.pem(),
        server_key_pem: server_key.serialize_pem(),
        client_cert_pem: client_cert.pem(),
        client_key_pem: client_key.serialize_pem(),
        client_cn: client_cn.to_string(),
    })
}

// ── EC2 Helpers ──────────────────────────────────────────────────────

/// Resolve the default VPC for the configured region. Both us-east-2 and
/// every Integrations_Tester region we use today have one — fail loudly if
/// not.
pub async fn default_vpc_id(ec2: &aws_sdk_ec2::Client) -> TestResult<String> {
    use aws_sdk_ec2::types::Filter;

    let resp = ec2
        .describe_vpcs()
        .filters(Filter::builder().name("is-default").values("true").build())
        .send()
        .await?;
    resp.vpcs()
        .iter()
        .find_map(|v| v.vpc_id().map(|s| s.to_string()))
        .ok_or_else(|| "no default VPC found in region".into())
}

pub async fn first_subnet_in_vpc(ec2: &aws_sdk_ec2::Client, vpc_id: &str) -> TestResult<String> {
    use aws_sdk_ec2::types::Filter;

    let resp = ec2
        .describe_subnets()
        .filters(Filter::builder().name("vpc-id").values(vpc_id).build())
        .send()
        .await?;
    resp.subnets()
        .iter()
        .find_map(|s| s.subnet_id().map(|s| s.to_string()))
        .ok_or_else(|| format!("no subnets in vpc {vpc_id}").into())
}

/// Find the most recent Canonical-published Ubuntu 24.04 LTS amd64 AMI.
pub async fn latest_ubuntu_noble_amd64_ami(ec2: &aws_sdk_ec2::Client) -> TestResult<String> {
    use aws_sdk_ec2::types::Filter;

    let resp = ec2
        .describe_images()
        .owners("099720109477") // Canonical
        .filters(
            Filter::builder()
                .name("name")
                .values("ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*")
                .build(),
        )
        .filters(
            Filter::builder()
                .name("virtualization-type")
                .values("hvm")
                .build(),
        )
        .send()
        .await?;

    let mut images: Vec<_> = resp.images().to_vec();
    images.sort_by(|a, b| a.creation_date().cmp(&b.creation_date()));
    images
        .last()
        .and_then(|i| i.image_id().map(|s| s.to_string()))
        .ok_or_else(|| "no Ubuntu Noble AMIs found".into())
}

/// Create a single-ingress-rule security group exposing one TCP port from any
/// source. Returns the SG id. Caller must register it with `AwsCleanupRegistry`.
pub async fn create_open_security_group(
    ec2: &aws_sdk_ec2::Client,
    vpc_id: &str,
    name: &str,
    ingress_ports: &[i32],
) -> TestResult<String> {
    use aws_sdk_ec2::types::{IpPermission, IpRange, ResourceType, Tag, TagSpecification};

    // Tag at creation so the IAM policy gating these tests can scope
    // DeleteSecurityGroup / AuthorizeSecurityGroupIngress by
    // `aws:ResourceTag/managed_by`.
    let tag_spec = TagSpecification::builder()
        .resource_type(ResourceType::SecurityGroup)
        .tags(Tag::builder().key("Name").value(name).build())
        .tags(
            Tag::builder()
                .key("managed_by")
                .value("clickhousectl_e2e")
                .build(),
        )
        .build();

    let sg = ec2
        .create_security_group()
        .vpc_id(vpc_id)
        .group_name(name)
        .description(format!("clickhousectl e2e ({name})"))
        .tag_specifications(tag_spec)
        .send()
        .await?;
    let sg_id = sg
        .group_id()
        .ok_or("CreateSecurityGroup returned no id")?
        .to_string();

    let permissions: Vec<IpPermission> = ingress_ports
        .iter()
        .map(|port| {
            IpPermission::builder()
                .ip_protocol("tcp")
                .from_port(*port)
                .to_port(*port)
                .ip_ranges(IpRange::builder().cidr_ip("0.0.0.0/0").build())
                .build()
        })
        .collect();

    ec2.authorize_security_group_ingress()
        .group_id(&sg_id)
        .set_ip_permissions(Some(permissions))
        .send()
        .await?;

    Ok(sg_id)
}

/// Launch a single Ubuntu EC2 instance with the given user_data script,
/// associated public IP, and security group. Returns `(instance_id, public_ip)`
/// once the instance is in `running` state. Caller must register the instance
/// in `AwsCleanupRegistry`.
pub async fn launch_ec2_instance(
    ec2: &aws_sdk_ec2::Client,
    ami_id: &str,
    subnet_id: &str,
    sg_id: &str,
    instance_type: &str,
    user_data: &str,
    name_tag: &str,
) -> TestResult<(String, String)> {
    use aws_sdk_ec2::types::{
        BlockDeviceMapping, EbsBlockDevice, InstanceNetworkInterfaceSpecification, InstanceType,
        ResourceType, Tag, TagSpecification, VolumeType,
    };
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;

    let ud_b64 = STANDARD.encode(user_data.as_bytes());

    let nic = InstanceNetworkInterfaceSpecification::builder()
        .device_index(0)
        .associate_public_ip_address(true)
        .subnet_id(subnet_id)
        .groups(sg_id)
        .build();

    let root_volume = BlockDeviceMapping::builder()
        .device_name("/dev/sda1")
        .ebs(
            EbsBlockDevice::builder()
                .volume_size(20)
                .volume_type(VolumeType::Gp3)
                .delete_on_termination(true)
                .build(),
        )
        .build();

    // Tag the instance and every resource RunInstances creates alongside it
    // (root volume, primary network interface). Volumes/NICs go away with the
    // instance, but tagging them at creation lets a tightly-scoped IAM policy
    // gate `aws:ResourceTag/managed_by` on every action against them.
    let instance_tags = TagSpecification::builder()
        .resource_type(ResourceType::Instance)
        .tags(Tag::builder().key("Name").value(name_tag).build())
        .tags(
            Tag::builder()
                .key("managed_by")
                .value("clickhousectl_e2e")
                .build(),
        )
        .build();
    let volume_tags = TagSpecification::builder()
        .resource_type(ResourceType::Volume)
        .tags(
            Tag::builder()
                .key("managed_by")
                .value("clickhousectl_e2e")
                .build(),
        )
        .build();
    let nic_tags = TagSpecification::builder()
        .resource_type(ResourceType::NetworkInterface)
        .tags(
            Tag::builder()
                .key("managed_by")
                .value("clickhousectl_e2e")
                .build(),
        )
        .build();

    let resp = ec2
        .run_instances()
        .image_id(ami_id)
        .instance_type(InstanceType::from(instance_type))
        .min_count(1)
        .max_count(1)
        .user_data(ud_b64)
        .network_interfaces(nic)
        .block_device_mappings(root_volume)
        .tag_specifications(instance_tags)
        .tag_specifications(volume_tags)
        .tag_specifications(nic_tags)
        .send()
        .await?;

    let instance = resp
        .instances()
        .first()
        .ok_or("RunInstances returned no instances")?;
    let instance_id = instance
        .instance_id()
        .ok_or("RunInstances returned instance without id")?
        .to_string();

    // Poll until running + public IP allocated.
    eprintln!("  waiting for ec2 instance to enter running state");
    let public_ip = poll_until(
        "ec2 instance running with public ip",
        Duration::from_secs(300),
        Duration::from_secs(5),
        || {
            let ec2 = ec2.clone();
            let instance_id = instance_id.clone();
            async move {
                let resp = ec2
                    .describe_instances()
                    .instance_ids(instance_id)
                    .send()
                    .await?;
                let inst = resp
                    .reservations()
                    .iter()
                    .flat_map(|r| r.instances())
                    .next();
                match inst {
                    None => Ok(None),
                    Some(i) => {
                        let state = i
                            .state()
                            .and_then(|s| s.name())
                            .map(|n| n.as_str().to_string())
                            .unwrap_or_default();
                        if state != "running" {
                            return Ok(None);
                        }
                        Ok(i.public_ip_address().map(|s| s.to_string()))
                    }
                }
            }
        },
    )
    .await?;

    Ok((instance_id, public_ip))
}

/// Allocate an Elastic IP. Returns `(public_ip, allocation_id)`. Caller is
/// responsible for `register_ec2_elastic_ip(allocation_id)` and for calling
/// `associate_elastic_ip` once an instance exists.
pub async fn allocate_elastic_ip(ec2: &aws_sdk_ec2::Client) -> TestResult<(String, String)> {
    use aws_sdk_ec2::types::{DomainType, ResourceType, Tag, TagSpecification};

    // Tag at creation so the IAM policy gating these tests can scope
    // ReleaseAddress / Associate / Disassociate by
    // `aws:ResourceTag/managed_by`.
    let tag_spec = TagSpecification::builder()
        .resource_type(ResourceType::ElasticIp)
        .tags(
            Tag::builder()
                .key("managed_by")
                .value("clickhousectl_e2e")
                .build(),
        )
        .build();

    let resp = ec2
        .allocate_address()
        .domain(DomainType::Vpc)
        .tag_specifications(tag_spec)
        .send()
        .await?;
    let ip = resp
        .public_ip()
        .ok_or("AllocateAddress returned no public_ip")?
        .to_string();
    let alloc = resp
        .allocation_id()
        .ok_or("AllocateAddress returned no allocation_id")?
        .to_string();
    Ok((ip, alloc))
}

pub async fn associate_elastic_ip(
    ec2: &aws_sdk_ec2::Client,
    allocation_id: &str,
    instance_id: &str,
) -> TestResult<()> {
    ec2.associate_address()
        .allocation_id(allocation_id)
        .instance_id(instance_id)
        .send()
        .await?;
    Ok(())
}

/// Poll Redpanda's admin API for an HTTP user-lookup endpoint to confirm a
/// SCRAM user has been provisioned. Lets us replace fixed-time sleeps in
/// kafka stages with a real readiness check.
pub async fn wait_for_redpanda_scram_user(
    host: &str,
    admin_port: u16,
    username: &str,
    timeout: Duration,
) -> TestResult<()> {
    let target = format!("http://{host}:{admin_port}/v1/security/users");
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    poll_until(
        &format!("redpanda scram user {username}"),
        timeout,
        Duration::from_secs(3),
        || {
            let http = http.clone();
            let target = target.clone();
            let username = username.to_string();
            async move {
                let resp = match http.get(&target).send().await {
                    Ok(r) => r,
                    Err(_) => return Ok(None),
                };
                if !resp.status().is_success() {
                    return Ok(None);
                }
                let body: serde_json::Value = match resp.json().await {
                    Ok(b) => b,
                    Err(_) => return Ok(None),
                };
                let found = body
                    .as_array()
                    .map(|users| users.iter().any(|u| u.as_str() == Some(&username)))
                    .unwrap_or(false);
                if found { Ok(Some(())) } else { Ok(None) }
            }
        },
    )
    .await
}

/// Stable TCP-port probe: succeed only when the port has been open for
/// `required_consecutive` checks in a row (5 s apart). Catches "port opens,
/// then closes briefly during a service restart, then opens again" patterns
/// that would slip past a single-success probe (e.g. MongoDB's bootstrap
/// restarts mongod with auth enabled mid-script).
pub async fn wait_for_stable_tcp_port(
    host: &str,
    port: u16,
    required_consecutive: u32,
    total_timeout: Duration,
) -> TestResult<()> {
    let target = format!("{host}:{port}");
    let consecutive = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    poll_until(
        &format!("stable tcp port {host}:{port} ({required_consecutive}× in a row)"),
        total_timeout,
        Duration::from_secs(5),
        || {
            let target = target.clone();
            let consecutive = consecutive.clone();
            async move {
                match tokio::time::timeout(
                    Duration::from_secs(3),
                    tokio::net::TcpStream::connect(&target),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        let n = consecutive.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        if n >= required_consecutive {
                            Ok(Some(()))
                        } else {
                            Ok(None)
                        }
                    }
                    _ => {
                        consecutive.store(0, std::sync::atomic::Ordering::Relaxed);
                        Ok(None)
                    }
                }
            }
        },
    )
    .await
}

/// Best-effort TCP connect probe — used to wait for a service (Redpanda, etc.)
/// to start listening after `user_data` finishes.
pub async fn wait_for_tcp_port(host: &str, port: u16, timeout: Duration) -> TestResult<()> {
    let target = format!("{host}:{port}");
    poll_until(
        &format!("tcp port {host}:{port}"),
        timeout,
        Duration::from_secs(5),
        || {
            let target = target.clone();
            async move {
                match tokio::time::timeout(
                    Duration::from_secs(3),
                    tokio::net::TcpStream::connect(&target),
                )
                .await
                {
                    Ok(Ok(_)) => Ok(Some(())),
                    _ => Ok(None),
                }
            }
        },
    )
    .await
}

async fn terminate_and_wait(ec2: &aws_sdk_ec2::Client, instance_ids: &[String]) -> TestResult<()> {
    if instance_ids.is_empty() {
        return Ok(());
    }
    eprintln!("  cleanup: terminating ec2 instances");
    let _ = ec2
        .terminate_instances()
        .set_instance_ids(Some(instance_ids.to_vec()))
        .send()
        .await?;

    // Poll until all are in 'terminated'. AWS deletes them lazily after that.
    poll_until(
        "ec2 instances terminated",
        Duration::from_secs(300),
        Duration::from_secs(10),
        || {
            let ec2 = ec2.clone();
            let ids = instance_ids.to_vec();
            async move {
                let resp = ec2
                    .describe_instances()
                    .set_instance_ids(Some(ids))
                    .send()
                    .await?;
                let all_terminated =
                    resp.reservations()
                        .iter()
                        .flat_map(|r| r.instances())
                        .all(|i| {
                            i.state()
                                .and_then(|s| s.name())
                                .map(|n| n.as_str() == "terminated")
                                .unwrap_or(false)
                        });
                if all_terminated {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            }
        },
    )
    .await
}

/// Create an IAM role whose only purpose is to be assumed by a ClickPipes
/// service principal. The trust policy targets `service_principal_arn` exactly
/// — no wildcards — so the role can only be assumed by this one CHC service.
///
/// Tags are attached best-effort after creation; the `Integrations_Tester` SSO
/// role can `CreateRole` but is denied `iam:TagRole`, so tagging at create-time
/// would fail the whole call. Cleanup is tracked via `AwsCleanupRegistry`, so
/// missing tags don't affect resource lifecycle.
pub async fn create_clickpipes_iam_role(
    iam: &aws_sdk_iam::Client,
    role_name: &str,
    service_principal_arn: &str,
    inline_policy_doc: &str,
    tags: &[(String, String)],
) -> TestResult<String> {
    use aws_sdk_iam::types::Tag;

    let trust_policy = serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [{
            "Effect": "Allow",
            "Principal": { "AWS": service_principal_arn },
            "Action": "sts:AssumeRole"
        }]
    })
    .to_string();

    // Tag at CreateRole so the role is never visible to AWS untagged. A
    // tightly-scoped IAM policy can require `aws:RequestTag/managed_by` on
    // iam:CreateRole, which this satisfies; the previous CreateRole→TagRole
    // pattern left a window where the role existed without the tag.
    let aws_tags: Vec<Tag> = tags
        .iter()
        .map(|(k, v)| Tag::builder().key(k).value(v).build())
        .collect::<Result<_, _>>()?;

    let resp = iam
        .create_role()
        .role_name(role_name)
        .assume_role_policy_document(trust_policy)
        .set_tags(if aws_tags.is_empty() {
            None
        } else {
            Some(aws_tags)
        })
        .send()
        .await?;
    let role_arn = resp
        .role()
        .map(|r| r.arn().to_string())
        .ok_or("CreateRole returned no role")?;

    iam.put_role_policy()
        .role_name(role_name)
        .policy_name(format!("{role_name}-inline"))
        .policy_document(inline_policy_doc)
        .send()
        .await?;

    Ok(role_arn)
}

// ── Kinesis Helpers ──────────────────────────────────────────────────

/// Create an on-demand Kinesis data stream with a single shard, wait for it to
/// enter `ACTIVE`, and tag it. Returns the stream's ARN.
pub async fn create_kinesis_stream(
    kinesis: &aws_sdk_kinesis::Client,
    stream_name: &str,
    tags: &[(String, String)],
) -> TestResult<String> {
    use aws_sdk_kinesis::types::{StreamMode, StreamModeDetails};

    kinesis
        .create_stream()
        .stream_name(stream_name)
        .stream_mode_details(
            StreamModeDetails::builder()
                .stream_mode(StreamMode::OnDemand)
                .build()?,
        )
        .send()
        .await?;

    // Wait for ACTIVE — on-demand streams typically reach ACTIVE within ~30s.
    let arn = poll_until(
        &format!("kinesis stream {stream_name} ACTIVE"),
        Duration::from_secs(300),
        Duration::from_secs(5),
        || {
            let kinesis = kinesis.clone();
            let stream_name = stream_name.to_string();
            async move {
                let resp = kinesis
                    .describe_stream_summary()
                    .stream_name(&stream_name)
                    .send()
                    .await?;
                let desc = resp
                    .stream_description_summary()
                    .ok_or("DescribeStreamSummary returned no summary")?;
                let status = desc.stream_status();
                if status.as_str() == "ACTIVE" {
                    Ok(Some(desc.stream_arn().to_string()))
                } else {
                    Ok(None)
                }
            }
        },
    )
    .await?;

    if !tags.is_empty() {
        let mut req = kinesis.add_tags_to_stream().stream_name(stream_name);
        for (k, v) in tags {
            req = req.tags(k, v);
        }
        // Fail loudly: an untagged stream can't be deleted by a tag-scoped
        // IAM policy and would leak past the test run.
        req.send().await?;
    }

    Ok(arn)
}

/// Put a single JSON-encoded record onto the stream. Partition key is the
/// caller's choice — any string is fine for a 1-shard stream.
pub async fn put_kinesis_record(
    kinesis: &aws_sdk_kinesis::Client,
    stream_name: &str,
    partition_key: &str,
    body: &[u8],
) -> TestResult<()> {
    kinesis
        .put_record()
        .stream_name(stream_name)
        .partition_key(partition_key)
        .data(aws_sdk_kinesis::primitives::Blob::new(body.to_vec()))
        .send()
        .await?;
    Ok(())
}

pub async fn delete_kinesis_stream(
    kinesis: &aws_sdk_kinesis::Client,
    stream_name: &str,
) -> TestResult<()> {
    eprintln!("  cleanup: deleting kinesis stream");
    match kinesis
        .delete_stream()
        .stream_name(stream_name)
        .enforce_consumer_deletion(true)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("ResourceNotFoundException") => Ok(()),
        Err(e) => Err(e.into()),
    }
}
