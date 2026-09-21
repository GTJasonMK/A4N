mod character;
mod material;
mod object;
mod portrait;
mod scene;

pub(in crate::project_dependency_graph) use character::character_profile_detail_records;
pub(in crate::project_dependency_graph) use material::profile_material_image_detail_records;
pub(in crate::project_dependency_graph) use object::object_profile_detail_records;
pub(in crate::project_dependency_graph) use portrait::character_portrait_detail_records;
pub(in crate::project_dependency_graph) use scene::scene_profile_detail_records;
