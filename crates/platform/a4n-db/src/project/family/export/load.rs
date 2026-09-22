macro_rules! project_family_export_load_by_ids {
    ($fn_name:ident, $entity:path, $filter_col:expr $(,)?) => {
        pub(super) async fn $fn_name<C>(
            db: &C,
            ids: &[i64],
        ) -> Result<
            Vec<<$entity as sea_orm::EntityTrait>::Model>,
            super::types::ProjectFamilyExportError,
        >
        where
            C: sea_orm::ConnectionTrait,
        {
            use sea_orm::{ColumnTrait as _, QueryFilter as _};

            if ids.is_empty() {
                return Ok(Vec::new());
            }
            <$entity as sea_orm::EntityTrait>::find()
                .filter($filter_col.is_in(ids.to_vec()))
                .all(db)
                .await
                .map_err(super::types::ProjectFamilyExportError::Database)
        }
    };
    ($fn_name:ident, $entity:path, $filter_col:expr, $first_order_col:expr $(, $order_col:expr)* $(,)?) => {
        pub(super) async fn $fn_name<C>(
            db: &C,
            ids: &[i64],
        ) -> Result<
            Vec<<$entity as sea_orm::EntityTrait>::Model>,
            super::types::ProjectFamilyExportError,
        >
        where
            C: sea_orm::ConnectionTrait,
        {
            use sea_orm::{ColumnTrait as _, QueryFilter as _, QueryOrder as _};

            if ids.is_empty() {
                return Ok(Vec::new());
            }
            let query = <$entity as sea_orm::EntityTrait>::find()
                .filter($filter_col.is_in(ids.to_vec()))
                .order_by_asc($first_order_col);
            $(
                let query = query.order_by_asc($order_col);
            )*
            query
                .all(db)
                .await
                .map_err(super::types::ProjectFamilyExportError::Database)
        }
    };
}
