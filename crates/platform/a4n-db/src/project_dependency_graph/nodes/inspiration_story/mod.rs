mod inspiration;
pub(in crate::project_dependency_graph) use inspiration::add_inspiration_nodes;

mod story;
pub(in crate::project_dependency_graph) use story::add_story_nodes;

#[cfg(test)]
mod tests;
