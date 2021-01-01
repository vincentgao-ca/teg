use async_graphql::{
    Context,
    FieldResult,
    ID,
};
use teg_material::Material;

use super::Toolhead;

#[async_graphql::Object]
impl Toolhead {
    async fn id(&self) -> ID {
        (&self.id).into()
    }

    async fn current_material<'ctx>(&self, ctx: &'ctx Context<'_>,) -> FieldResult<Option<Material>> {
        let db: &crate::Db = ctx.data()?;

        let material = if let Some(material_id) = self.model.material_id {
            let material = Material::get(&db, material_id).await?;

            Some(material)
        } else {
            None
        };

        Ok(material)
    }
}
