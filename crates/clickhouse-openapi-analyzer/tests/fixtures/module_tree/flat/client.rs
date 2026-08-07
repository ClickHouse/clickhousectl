pub struct Client;

impl Client {
    pub async fn list_widgets(
        &self,
        state: Option<&WidgetState>,
    ) -> Result<WidgetAlias, Error> {
        unimplemented!()
    }
}
