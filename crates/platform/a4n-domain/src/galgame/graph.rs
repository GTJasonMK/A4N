use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const GALGAME_STORY_GRAPH_SCHEMA_VERSION: &str = "galgame_story_graph_v1";
pub const GALGAME_RUNTIME_MAX_STEPS: u32 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StoryNodeKind {
    Narrative,
    Choice,
    Fork,
    Join,
    LoopEntry,
    LoopExit,
    ChapterBoundary,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StoryEdgeKind {
    Sequence,
    Choice,
    LoopBack,
    ChapterTransition,
    Interrupt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum StoryValue {
    Bool(bool),
    Int(i64),
    String(String),
    Enum(String),
}

impl StoryValue {
    fn kind(&self) -> &'static str {
        match self {
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::String(_) => "string",
            Self::Enum(_) => "enum",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum StoryExpression {
    True,
    False,
    Variable(String),
    Constant(StoryValue),
    Not(Box<Self>),
    All(Vec<Self>),
    Any(Vec<Self>),
    Equals(Box<Self>, Box<Self>),
    NotEquals(Box<Self>, Box<Self>),
    GreaterThan(Box<Self>, Box<Self>),
    LessThan(Box<Self>, Box<Self>),
    HasKnowledge(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum StoryStateOperation {
    Set { key: String, value: StoryValue },
    Increment { key: String, amount: i64 },
    AddKnowledge { key: String },
    RemoveKnowledge { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryStateVariable {
    pub key: String,
    pub value_kind: String,
    pub default_value: StoryValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StorySourceRef {
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_span_ids: Vec<String>,
}

impl StorySourceRef {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("root_project_id", self.root_project_id),
            ("source_snapshot_id", self.source_snapshot_id),
            (
                "source_snapshot_chapter_id",
                self.source_snapshot_chapter_id,
            ),
        ] {
            if value <= 0 {
                return Err(format!("{name} 必须大于 0"));
            }
        }
        if let Some(source_chapter_id) = self.source_chapter_id
            && source_chapter_id <= 0
        {
            return Err("source_chapter_id 必须大于 0".into());
        }
        if self.source_span_ids.iter().any(|id| id.trim().is_empty()) {
            return Err("source_span_ids 不能包含空值".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryNarrativeLine {
    pub id: String,
    pub display_order: i32,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub presentation: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryNode {
    pub id: String,
    pub kind: StoryNodeKind,
    pub title: String,
    pub summary: String,
    pub primary_source: Option<StorySourceRef>,
    pub source_refs: Vec<StorySourceRef>,
    pub lines: Vec<StoryNarrativeLine>,
    pub state_before: BTreeMap<String, StoryValue>,
    pub state_after: BTreeMap<String, StoryValue>,
    pub state_operations: Vec<StoryStateOperation>,
    pub media_intents: Vec<serde_json::Value>,
    pub layout_x: f64,
    pub layout_y: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryEdge {
    pub id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub from_port: String,
    pub to_port: String,
    pub kind: StoryEdgeKind,
    pub branch_group_id: Option<String>,
    pub guard: Option<StoryExpression>,
    pub state_operations: Vec<StoryStateOperation>,
    pub display_order: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryBranchOption {
    pub id: String,
    pub label: String,
    pub edge_id: String,
    pub condition: Option<StoryExpression>,
    pub state_operations: Vec<StoryStateOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryBranchGroup {
    pub id: String,
    pub fork_node_id: String,
    pub options: Vec<StoryBranchOption>,
    pub join_node_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryLoopGroup {
    pub id: String,
    pub entry_node_id: String,
    pub body_exit_node_id: String,
    pub back_edge_id: String,
    pub back_to_node_id: String,
    pub exit_node_id: String,
    pub guard: StoryExpression,
    pub state_operations: Vec<StoryStateOperation>,
    pub max_iterations: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryGraph {
    pub schema_version: String,
    pub id: String,
    pub project_id: i64,
    pub source_snapshot_id: i64,
    pub revision: i64,
    pub entry_node_id: String,
    pub terminal_node_ids: Vec<String>,
    pub nodes: Vec<StoryNode>,
    pub edges: Vec<StoryEdge>,
    pub branch_groups: Vec<StoryBranchGroup>,
    pub loop_groups: Vec<StoryLoopGroup>,
    pub state_variables: Vec<StoryStateVariable>,
}

impl StoryGraph {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != GALGAME_STORY_GRAPH_SCHEMA_VERSION {
            return Err(format!(
                "schema_version 必须是 {GALGAME_STORY_GRAPH_SCHEMA_VERSION}"
            ));
        }
        if self.id.trim().is_empty() || self.project_id <= 0 || self.source_snapshot_id <= 0 {
            return Err("图身份和来源快照必须有效".into());
        }
        if self.revision <= 0 {
            return Err("revision 必须大于 0".into());
        }
        let nodes = unique_ids(self.nodes.iter().map(|node| node.id.as_str()), "节点")?;
        let edges = unique_ids(self.edges.iter().map(|edge| edge.id.as_str()), "边")?;
        let terminals = unique_ids(
            self.terminal_node_ids.iter().map(String::as_str),
            "终止节点",
        )?;
        if !nodes.contains(&self.entry_node_id) {
            return Err("entry_node_id 不存在".into());
        }
        validate_state_variables(&self.state_variables)?;
        let variable_kinds = state_variable_kinds(self);
        let node_by_id = self
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect::<HashMap<_, _>>();
        let mut outgoing_counts = HashMap::<&str, usize>::new();
        let mut incoming_counts = HashMap::<&str, usize>::new();
        let mut adjacency = HashMap::<&str, Vec<(&str, StoryEdgeKind)>>::new();
        let mut narrative_line_ids = HashSet::new();
        for node in &self.nodes {
            if node.id.trim().is_empty() || node.title.trim().is_empty() {
                return Err("节点 id 和 title 不能为空".into());
            }
            if let Some(source) = &node.primary_source {
                source.validate()?;
            }
            for source in &node.source_refs {
                source.validate()?;
            }
            for source in node.primary_source.iter().chain(node.source_refs.iter()) {
                if source.source_snapshot_id != self.source_snapshot_id {
                    return Err(format!("节点来源快照与图不一致: {}", node.id));
                }
            }
            validate_state_snapshot(&node.state_before, &variable_kinds)?;
            validate_state_snapshot(&node.state_after, &variable_kinds)?;
            validate_operations(&node.state_operations, &variable_kinds)?;
            unique_ids(node.lines.iter().map(|line| line.id.as_str()), "叙事行")?;
            for line in &node.lines {
                if !narrative_line_ids.insert(line.id.as_str()) {
                    return Err(format!("叙事行 id 必须在整张图中唯一: {}", line.id));
                }
                if crate::galgame::GalgameLineType::parse(&line.line_type).is_none() {
                    return Err(format!("叙事行类型无效: {}", line.line_type));
                }
                if line
                    .speaker_profile_id
                    .is_some_and(|profile_id| profile_id <= 0)
                {
                    return Err(format!("叙事行角色档案 id 必须为正整数: {}", line.id));
                }
            }
            let line_orders = node
                .lines
                .iter()
                .map(|line| line.display_order)
                .collect::<HashSet<_>>();
            if node.lines.iter().any(|line| line.display_order < 0)
                || line_orders.len() != node.lines.len()
            {
                return Err(format!("节点台词顺序不能小于 0: {}", node.id));
            }
        }
        for edge in &self.edges {
            if !nodes.contains(&edge.from_node_id) || !nodes.contains(&edge.to_node_id) {
                return Err(format!("边引用了不存在的节点: {}", edge.id));
            }
            if edge.from_port.trim().is_empty()
                || edge.to_port.trim().is_empty()
                || edge.display_order < 0
            {
                return Err(format!("边端口或顺序无效: {}", edge.id));
            }
            if edge.from_node_id == edge.to_node_id && edge.kind != StoryEdgeKind::LoopBack {
                return Err(format!("只有 loop_back 边可以自环: {}", edge.id));
            }
            validate_boolean_expression(edge.guard.as_ref(), &variable_kinds)?;
            validate_operations(&edge.state_operations, &variable_kinds)?;
            *outgoing_counts
                .entry(edge.from_node_id.as_str())
                .or_default() += 1;
            *incoming_counts.entry(edge.to_node_id.as_str()).or_default() += 1;
            adjacency
                .entry(edge.from_node_id.as_str())
                .or_default()
                .push((edge.to_node_id.as_str(), edge.kind));
        }
        let branch_ids = unique_ids(
            self.branch_groups.iter().map(|group| group.id.as_str()),
            "分叉组",
        )?;
        let mut branch_edge_ids = HashSet::new();
        let mut branch_fork_ids = HashSet::new();
        for branch in &self.branch_groups {
            let Some(fork) = node_by_id.get(branch.fork_node_id.as_str()) else {
                return Err(format!("分叉组引用了不存在的 fork 节点: {}", branch.id));
            };
            if !matches!(fork.kind, StoryNodeKind::Fork | StoryNodeKind::Choice) {
                return Err(format!("分叉组来源必须是 fork/choice 节点: {}", branch.id));
            }
            if !branch_fork_ids.insert(branch.fork_node_id.as_str()) {
                return Err(format!(
                    "每个分叉节点只能有一个分叉组: {}",
                    branch.fork_node_id
                ));
            }
            if branch.options.len() < 2 {
                return Err(format!("分叉组至少需要两个选项: {}", branch.id));
            }
            unique_ids(
                branch.options.iter().map(|option| option.id.as_str()),
                "分支选项",
            )?;
            for option in &branch.options {
                let Some(edge) = self.edges.iter().find(|edge| edge.id == option.edge_id) else {
                    return Err(format!("分支选项引用了不存在的边: {}", option.id));
                };
                if edge.kind != StoryEdgeKind::Choice
                    || edge.from_node_id != branch.fork_node_id
                    || edge.branch_group_id.as_deref() != Some(branch.id.as_str())
                {
                    return Err(format!("分支选项与 Choice 边不一致: {}", option.id));
                }
                if !branch_edge_ids.insert(option.edge_id.as_str()) {
                    return Err(format!("Choice 边被多个分支选项引用: {}", option.edge_id));
                }
                if option.label.trim().is_empty() {
                    return Err(format!("分支选项文本不能为空: {}", option.id));
                }
                validate_boolean_expression(option.condition.as_ref(), &variable_kinds)?;
                validate_operations(&option.state_operations, &variable_kinds)?;
            }
            if let Some(join) = &branch.join_node_id {
                if !node_by_id
                    .get(join.as_str())
                    .is_some_and(|node| node.kind == StoryNodeKind::Join)
                {
                    return Err(format!(
                        "分叉组 join_node_id 必须引用 Join 节点: {}",
                        branch.id
                    ));
                }
            }
        }
        for edge in &self.edges {
            match edge.kind {
                StoryEdgeKind::Choice => {
                    let Some(group_id) = edge.branch_group_id.as_deref() else {
                        return Err(format!("Choice 边缺少分叉组: {}", edge.id));
                    };
                    if !branch_ids.contains(group_id) || !branch_edge_ids.contains(edge.id.as_str())
                    {
                        return Err(format!("Choice 边未被对应分叉组选项引用: {}", edge.id));
                    }
                }
                _ if edge.branch_group_id.is_some() => {
                    return Err(format!("非 Choice 边不能引用分叉组: {}", edge.id));
                }
                _ => {}
            }
        }
        for node in &self.nodes {
            if matches!(node.kind, StoryNodeKind::Fork | StoryNodeKind::Choice)
                && !self
                    .branch_groups
                    .iter()
                    .any(|group| group.fork_node_id == node.id)
            {
                return Err(format!("Fork/Choice 节点缺少显式分叉组: {}", node.id));
            }
        }
        unique_ids(
            self.loop_groups.iter().map(|group| group.id.as_str()),
            "循环组",
        )?;
        let declared_loop_edges = self
            .loop_groups
            .iter()
            .map(|group| group.back_edge_id.as_str())
            .collect::<HashSet<_>>();
        for edge in self
            .edges
            .iter()
            .filter(|edge| edge.kind == StoryEdgeKind::LoopBack)
        {
            if !declared_loop_edges.contains(edge.id.as_str()) {
                return Err(format!("loop_back 边未归属任何 LoopGroup: {}", edge.id));
            }
        }
        let mut declared_loop_edge_ids = HashSet::new();
        for group in &self.loop_groups {
            for (name, node_id) in [
                ("entry_node_id", &group.entry_node_id),
                ("body_exit_node_id", &group.body_exit_node_id),
                ("back_to_node_id", &group.back_to_node_id),
                ("exit_node_id", &group.exit_node_id),
            ] {
                if !nodes.contains(node_id) {
                    return Err(format!("循环组 {name} 不存在: {}", group.id));
                }
            }
            if !edges.contains(&group.back_edge_id) {
                return Err(format!("循环组 back_edge 不存在: {}", group.id));
            }
            if !declared_loop_edge_ids.insert(group.back_edge_id.as_str()) {
                return Err(format!(
                    "loop_back 边重复归属循环组: {}",
                    group.back_edge_id
                ));
            }
            let back_edge = self
                .edges
                .iter()
                .find(|edge| edge.id == group.back_edge_id)
                .expect("checked above");
            if back_edge.kind != StoryEdgeKind::LoopBack
                || back_edge.from_node_id != group.body_exit_node_id
                || back_edge.to_node_id != group.back_to_node_id
                || group.back_to_node_id != group.entry_node_id
            {
                return Err(format!("循环组 back_edge 与循环边界不一致: {}", group.id));
            }
            if !self.edges.iter().any(|edge| {
                edge.from_node_id == group.body_exit_node_id
                    && edge.to_node_id == group.exit_node_id
                    && edge.kind != StoryEdgeKind::LoopBack
            }) {
                return Err(format!(
                    "循环组 body_exit 必须能连接到 exit_node: {}",
                    group.id
                ));
            }
            if !self.edges.iter().any(|edge| {
                edge.from_node_id == group.entry_node_id
                    && edge.to_node_id == group.body_exit_node_id
                    && edge.kind != StoryEdgeKind::LoopBack
            }) {
                return Err(format!("循环组 entry 必须连接到 body_exit: {}", group.id));
            }
            if node_by_id[group.entry_node_id.as_str()].kind != StoryNodeKind::LoopEntry
                || node_by_id[group.exit_node_id.as_str()].kind != StoryNodeKind::LoopExit
            {
                return Err(format!(
                    "循环组必须使用 LoopEntry 和 LoopExit 节点: {}",
                    group.id
                ));
            }
            if group.max_iterations.is_none_or(|limit| limit == 0) {
                return Err(format!(
                    "循环组必须设置大于 0 的 max_iterations: {}",
                    group.id
                ));
            }
            validate_boolean_expression(Some(&group.guard), &variable_kinds)?;
            validate_operations(&group.state_operations, &variable_kinds)?;
        }
        for node in &self.nodes {
            if (node.kind == StoryNodeKind::LoopEntry
                && !self
                    .loop_groups
                    .iter()
                    .any(|group| group.entry_node_id == node.id))
                || (node.kind == StoryNodeKind::LoopExit
                    && !self
                        .loop_groups
                        .iter()
                        .any(|group| group.exit_node_id == node.id))
            {
                return Err(format!("循环边界节点必须属于 LoopGroup: {}", node.id));
            }
            let outgoing = outgoing_counts
                .get(node.id.as_str())
                .copied()
                .unwrap_or_default();
            let incoming = incoming_counts
                .get(node.id.as_str())
                .copied()
                .unwrap_or_default();
            let outgoing_edges = self.outgoing_edges(&node.id);
            let incoming_edges = self
                .edges
                .iter()
                .filter(|edge| edge.to_node_id == node.id)
                .collect::<Vec<_>>();
            let is_entry = node.id == self.entry_node_id;
            if (node.kind == StoryNodeKind::Terminal) != (outgoing == 0) {
                return Err(format!(
                    "只有无输出节点可以是 Terminal，且所有终点必须显式标记: {}",
                    node.id
                ));
            }
            match node.kind {
                StoryNodeKind::Terminal => {
                    let expected_inputs = if is_entry { 0 } else { 1 };
                    if incoming != expected_inputs {
                        return Err(format!("终止节点必须只有一个输入: {}", node.id));
                    }
                }
                StoryNodeKind::Fork | StoryNodeKind::Choice => {
                    let option_count = self
                        .branch_groups
                        .iter()
                        .find(|group| group.fork_node_id == node.id)
                        .map_or(0, |group| group.options.len());
                    let expected_inputs = if is_entry { 0 } else { 1 };
                    if incoming != expected_inputs
                        || outgoing != option_count
                        || outgoing_edges
                            .iter()
                            .any(|edge| edge.kind != StoryEdgeKind::Choice)
                    {
                        return Err(format!(
                            "分叉节点必须有至多一个输入，且所有输出都属于显式分支组: {}",
                            node.id
                        ));
                    }
                }
                StoryNodeKind::Join => {
                    if is_entry || incoming < 2 || outgoing != 1 {
                        return Err(format!(
                            "Join 节点必须有至少两个输入、一个输出，且不能是入口: {}",
                            node.id
                        ));
                    }
                }
                StoryNodeKind::LoopEntry => {
                    let loop_back_inputs = incoming_edges
                        .iter()
                        .filter(|edge| edge.kind == StoryEdgeKind::LoopBack)
                        .count();
                    let expected_inputs = if is_entry { 1 } else { 2 };
                    if incoming != expected_inputs || loop_back_inputs != 1 || outgoing != 1 {
                        return Err(format!(
                            "LoopEntry 必须有一个循环回边、一个循环体输出和至多一个外部输入: {}",
                            node.id
                        ));
                    }
                }
                StoryNodeKind::LoopExit => {
                    if is_entry || incoming != 1 || outgoing != 1 {
                        return Err(format!("LoopExit 必须有一个输入和一个输出: {}", node.id));
                    }
                }
                StoryNodeKind::Narrative | StoryNodeKind::ChapterBoundary => {
                    let body_exit = self
                        .loop_groups
                        .iter()
                        .find(|group| group.body_exit_node_id == node.id);
                    let expected_outgoing = if body_exit.is_some() { 2 } else { 1 };
                    let expected_inputs = if is_entry { 0 } else { 1 };
                    if incoming != expected_inputs || outgoing != expected_outgoing {
                        return Err(format!(
                            "普通节点必须单进单出；循环体出口必须单进、双出: {}",
                            node.id
                        ));
                    }
                    if let Some(group) = body_exit {
                        let has_back_edge = outgoing_edges
                            .iter()
                            .any(|edge| edge.id == group.back_edge_id);
                        let has_exit_edge = outgoing_edges.iter().any(|edge| {
                            edge.to_node_id == group.exit_node_id
                                && edge.kind != StoryEdgeKind::LoopBack
                        });
                        if !has_back_edge || !has_exit_edge {
                            return Err(format!(
                                "循环体出口必须连接循环回边和 LoopExit: {}",
                                node.id
                            ));
                        }
                    }
                }
            }
        }
        let actual_terminals = self
            .nodes
            .iter()
            .filter(|node| node.kind == StoryNodeKind::Terminal)
            .map(|node| node.id.as_str())
            .collect::<HashSet<_>>();
        if actual_terminals.len() != terminals.len()
            || !actual_terminals
                .iter()
                .all(|node_id| terminals.contains(*node_id))
        {
            return Err("terminal_node_ids 必须完整且只包含 Terminal 节点".into());
        }
        reject_unmarked_cycles(&adjacency)?;
        let mut reachable = HashSet::new();
        let mut pending = vec![self.entry_node_id.as_str()];
        while let Some(node_id) = pending.pop() {
            if !reachable.insert(node_id) {
                continue;
            }
            pending.extend(
                adjacency
                    .get(node_id)
                    .into_iter()
                    .flatten()
                    .map(|(next, _)| *next),
            );
        }
        if reachable.len() != self.nodes.len() {
            return Err("存在从 entry_node_id 不可达的节点".into());
        }
        Ok(())
    }

    pub fn outgoing_edges(&self, node_id: &str) -> Vec<&StoryEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.from_node_id == node_id)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoryRuntimeState {
    pub graph_id: String,
    pub graph_revision: i64,
    pub current_node_id: String,
    pub variables: BTreeMap<String, StoryValue>,
    pub player_knowledge: BTreeSet<String>,
    pub visited_counts: BTreeMap<String, u32>,
    pub loop_stack: Vec<String>,
    #[serde(default)]
    pub loop_iterations: BTreeMap<String, u32>,
    pub step_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryRuntimeTransition {
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_id: String,
    pub available_edge_ids: Vec<String>,
}

impl StoryRuntimeState {
    pub fn new(graph: &StoryGraph) -> Result<Self, String> {
        graph.validate()?;
        let mut variables = BTreeMap::new();
        for variable in &graph.state_variables {
            variables.insert(variable.key.clone(), variable.default_value.clone());
        }
        let mut state = Self {
            graph_id: graph.id.clone(),
            graph_revision: graph.revision,
            current_node_id: graph.entry_node_id.clone(),
            variables,
            player_knowledge: BTreeSet::new(),
            visited_counts: BTreeMap::new(),
            loop_stack: Vec::new(),
            loop_iterations: BTreeMap::new(),
            step_index: 0,
        };
        state.visited_counts.insert(graph.entry_node_id.clone(), 1);
        let entry = graph
            .nodes
            .iter()
            .find(|node| node.id == graph.entry_node_id)
            .expect("graph validation checked entry node");
        apply_operations(
            &mut state.variables,
            &mut state.player_knowledge,
            &entry.state_operations,
        )?;
        Ok(state)
    }

    pub fn available_edges<'a>(&self, graph: &'a StoryGraph) -> Result<Vec<&'a StoryEdge>, String> {
        self.ensure_graph(graph)?;
        Ok(graph
            .outgoing_edges(&self.current_node_id)
            .into_iter()
            .filter(|edge| edge_available(graph, edge, self))
            .collect())
    }

    pub fn advance(
        &mut self,
        graph: &StoryGraph,
        edge_id: &str,
    ) -> Result<StoryRuntimeTransition, String> {
        let available = self.available_edges(graph)?;
        let edge = available
            .into_iter()
            .find(|edge| edge.id == edge_id)
            .cloned()
            .ok_or_else(|| format!("边不可用或条件不满足: {edge_id}"))?;
        if self.step_index >= GALGAME_RUNTIME_MAX_STEPS {
            return Err(format!("运行步数超过上限: {GALGAME_RUNTIME_MAX_STEPS}"));
        }
        validate_operations(&edge.state_operations, &state_variable_kinds(graph))?;
        let mut next_state = self.clone();
        let from_node_id = next_state.current_node_id.clone();
        apply_operations(
            &mut next_state.variables,
            &mut next_state.player_knowledge,
            &edge.state_operations,
        )?;
        if let Some(option) = branch_option_for_edge(graph, &edge.id) {
            apply_operations(
                &mut next_state.variables,
                &mut next_state.player_knowledge,
                &option.state_operations,
            )?;
        }
        if edge.kind == StoryEdgeKind::LoopBack {
            let group = graph
                .loop_groups
                .iter()
                .find(|group| group.back_edge_id == edge.id)
                .expect("graph validation checked loop group");
            apply_operations(
                &mut next_state.variables,
                &mut next_state.player_knowledge,
                &group.state_operations,
            )?;
            *next_state
                .loop_iterations
                .entry(group.id.clone())
                .or_default() += 1;
            if next_state.loop_stack.last() != Some(&group.id) {
                next_state.loop_stack.push(group.id.clone());
            }
        }
        for group in &graph.loop_groups {
            if group.exit_node_id == edge.to_node_id
                && next_state.loop_stack.last() == Some(&group.id)
            {
                next_state.loop_stack.pop();
            }
        }
        next_state.current_node_id = edge.to_node_id.clone();
        next_state.step_index += 1;
        *next_state
            .visited_counts
            .entry(next_state.current_node_id.clone())
            .or_default() += 1;
        let target_node = graph
            .nodes
            .iter()
            .find(|node| node.id == next_state.current_node_id)
            .expect("graph validation checked edge target");
        apply_operations(
            &mut next_state.variables,
            &mut next_state.player_knowledge,
            &target_node.state_operations,
        )?;
        let transition = StoryRuntimeTransition {
            from_node_id,
            to_node_id: edge.to_node_id.clone(),
            edge_id: edge.id.clone(),
            available_edge_ids: graph
                .outgoing_edges(&next_state.current_node_id)
                .iter()
                .filter(|next| edge_available(graph, next, &next_state))
                .map(|next| next.id.clone())
                .collect(),
        };
        *self = next_state;
        Ok(transition)
    }

    fn ensure_graph(&self, graph: &StoryGraph) -> Result<(), String> {
        if self.graph_id != graph.id || self.graph_revision != graph.revision {
            return Err("运行状态所属图版本与当前图不一致".into());
        }
        if self.step_index > GALGAME_RUNTIME_MAX_STEPS {
            return Err(format!("运行状态步数超过上限: {}", self.step_index));
        }
        if !graph
            .nodes
            .iter()
            .any(|node| node.id == self.current_node_id)
        {
            return Err(format!("当前节点不存在: {}", self.current_node_id));
        }
        let variable_kinds = state_variable_kinds(graph);
        if self.variables.len() != variable_kinds.len()
            || self.variables.iter().any(|(key, value)| {
                variable_kinds
                    .get(key.as_str())
                    .is_none_or(|expected| *expected != value.kind())
            })
        {
            return Err("运行状态包含缺失或类型不匹配的状态变量".into());
        }
        let node_ids = graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<HashSet<_>>();
        if self
            .visited_counts
            .iter()
            .any(|(node_id, count)| !node_ids.contains(node_id.as_str()) || *count == 0)
        {
            return Err("运行状态访问记录包含无效节点".into());
        }
        if self
            .visited_counts
            .get(&self.current_node_id)
            .is_none_or(|count| *count == 0)
            || self
                .player_knowledge
                .iter()
                .any(|key| key.trim().is_empty())
        {
            return Err("运行状态当前节点或知识状态无效".into());
        }
        let loop_ids = graph
            .loop_groups
            .iter()
            .map(|group| group.id.as_str())
            .collect::<HashSet<_>>();
        if self
            .loop_stack
            .iter()
            .any(|id| !loop_ids.contains(id.as_str()))
            || self.loop_iterations.iter().any(|(id, count)| {
                let Some(group) = graph.loop_groups.iter().find(|group| group.id == *id) else {
                    return true;
                };
                *count == 0 || group.max_iterations.is_some_and(|limit| *count > limit)
            })
        {
            return Err("运行状态包含无效的循环记录".into());
        }
        Ok(())
    }
}

fn loop_edge_allowed(graph: &StoryGraph, edge: &StoryEdge, state: &StoryRuntimeState) -> bool {
    if edge.kind != StoryEdgeKind::LoopBack {
        return true;
    }
    let Some(group) = graph
        .loop_groups
        .iter()
        .find(|group| group.back_edge_id == edge.id)
    else {
        return false;
    };
    if !expression_matches(Some(&group.guard), state) {
        return false;
    }
    group.max_iterations.is_none_or(|limit| {
        state
            .loop_iterations
            .get(&group.id)
            .copied()
            .unwrap_or_default()
            < limit
    })
}

fn edge_available(graph: &StoryGraph, edge: &StoryEdge, state: &StoryRuntimeState) -> bool {
    expression_matches(edge.guard.as_ref(), state)
        && loop_edge_allowed(graph, edge, state)
        && branch_option_for_edge(graph, &edge.id)
            .is_none_or(|option| expression_matches(option.condition.as_ref(), state))
}

fn branch_option_for_edge<'a>(
    graph: &'a StoryGraph,
    edge_id: &str,
) -> Option<&'a StoryBranchOption> {
    graph
        .branch_groups
        .iter()
        .flat_map(|group| &group.options)
        .find(|option| option.edge_id == edge_id)
}

fn unique_ids<'a, I>(ids: I, label: &str) -> Result<HashSet<String>, String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut result = HashSet::new();
    for id in ids {
        if id.trim().is_empty() || !result.insert(id.to_string()) {
            return Err(format!("{label} id 不能为空且不能重复: {id}"));
        }
    }
    Ok(result)
}

fn validate_state_variables(variables: &[StoryStateVariable]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for variable in variables {
        if variable.key.trim().is_empty() || !ids.insert(&variable.key) {
            return Err(format!("状态变量 key 不能为空且不能重复: {}", variable.key));
        }
        if variable.default_value.kind() != variable.value_kind {
            return Err(format!(
                "状态变量默认值类型不匹配: key={}, expected={}, actual={}",
                variable.key,
                variable.value_kind,
                variable.default_value.kind()
            ));
        }
    }
    Ok(())
}

fn validate_state_snapshot(
    snapshot: &BTreeMap<String, StoryValue>,
    variables: &HashMap<&str, &str>,
) -> Result<(), String> {
    for (key, value) in snapshot {
        let Some(expected_kind) = variables.get(key.as_str()) else {
            return Err(format!("节点状态快照引用未声明变量: {key}"));
        };
        if *expected_kind != value.kind() {
            return Err(format!(
                "节点状态快照类型不匹配: key={key}, expected={expected_kind}, actual={}",
                value.kind()
            ));
        }
    }
    Ok(())
}

fn validate_boolean_expression(
    expression: Option<&StoryExpression>,
    variables: &HashMap<&str, &str>,
) -> Result<(), String> {
    let Some(expression) = expression else {
        return Ok(());
    };
    if expression_kind(expression, variables)? != "bool" {
        return Err("条件表达式必须返回 bool".into());
    }
    Ok(())
}

fn expression_kind<'a>(
    expression: &'a StoryExpression,
    variables: &'a HashMap<&str, &str>,
) -> Result<&'a str, String> {
    match expression {
        StoryExpression::True | StoryExpression::False => Ok("bool"),
        StoryExpression::HasKnowledge(key) => {
            if key.trim().is_empty() {
                return Err("HasKnowledge 的 key 不能为空".into());
            }
            Ok("bool")
        }
        StoryExpression::Constant(value) => Ok(value.kind()),
        StoryExpression::Variable(key) => variables
            .get(key.as_str())
            .copied()
            .ok_or_else(|| format!("表达式引用了未声明状态变量: {key}")),
        StoryExpression::Not(inner) => {
            require_expression_kind(inner, variables, "bool")?;
            Ok("bool")
        }
        StoryExpression::All(items) | StoryExpression::Any(items) => {
            for item in items {
                require_expression_kind(item, variables, "bool")?;
            }
            Ok("bool")
        }
        StoryExpression::Equals(left, right) | StoryExpression::NotEquals(left, right) => {
            let left_kind = expression_kind(left, variables)?;
            let right_kind = expression_kind(right, variables)?;
            if left_kind != right_kind {
                return Err(format!(
                    "相等比较的表达式类型不一致: left={left_kind}, right={right_kind}"
                ));
            }
            Ok("bool")
        }
        StoryExpression::GreaterThan(left, right) | StoryExpression::LessThan(left, right) => {
            require_expression_kind(left, variables, "int")?;
            require_expression_kind(right, variables, "int")?;
            Ok("bool")
        }
    }
}

fn require_expression_kind<'a>(
    expression: &'a StoryExpression,
    variables: &'a HashMap<&str, &str>,
    expected_kind: &str,
) -> Result<(), String> {
    let actual_kind = expression_kind(expression, variables)?;
    if actual_kind != expected_kind {
        return Err(format!(
            "表达式类型错误: expected={expected_kind}, actual={actual_kind}"
        ));
    }
    Ok(())
}

fn validate_operations(
    operations: &[StoryStateOperation],
    variables: &HashMap<&str, &str>,
) -> Result<(), String> {
    for operation in operations {
        let (key, expected_kind) = match operation {
            StoryStateOperation::Set { key, value } => (key, value.kind()),
            StoryStateOperation::Increment { key, .. } => (key, "int"),
            StoryStateOperation::AddKnowledge { key }
            | StoryStateOperation::RemoveKnowledge { key } => {
                if key.trim().is_empty() {
                    return Err("知识状态 key 不能为空".into());
                }
                continue;
            }
        };
        let Some(actual_kind) = variables.get(key.as_str()) else {
            return Err(format!("状态操作引用了未声明变量: {key}"));
        };
        if *actual_kind != expected_kind {
            return Err(format!(
                "状态操作类型不匹配: key={key}, expected={actual_kind}, actual={expected_kind}"
            ));
        }
    }
    Ok(())
}

fn state_variable_kinds(graph: &StoryGraph) -> HashMap<&str, &str> {
    graph
        .state_variables
        .iter()
        .map(|variable| (variable.key.as_str(), variable.value_kind.as_str()))
        .collect()
}

fn expression_matches(expression: Option<&StoryExpression>, state: &StoryRuntimeState) -> bool {
    let Some(expression) = expression else {
        return true;
    };
    match expression {
        StoryExpression::True => true,
        StoryExpression::False => false,
        StoryExpression::Variable(key) => state
            .variables
            .get(key)
            .is_some_and(|value| matches!(value, StoryValue::Bool(true))),
        StoryExpression::Constant(StoryValue::Bool(value)) => *value,
        StoryExpression::Constant(_) => false,
        StoryExpression::Not(inner) => !expression_matches(Some(inner), state),
        StoryExpression::All(items) => items
            .iter()
            .all(|item| expression_matches(Some(item), state)),
        StoryExpression::Any(items) => items
            .iter()
            .any(|item| expression_matches(Some(item), state)),
        StoryExpression::HasKnowledge(key) => state.player_knowledge.contains(key),
        StoryExpression::Equals(left, right) => {
            evaluate_value(left, state) == evaluate_value(right, state)
        }
        StoryExpression::NotEquals(left, right) => {
            evaluate_value(left, state) != evaluate_value(right, state)
        }
        StoryExpression::GreaterThan(left, right) => compare_ints(left, right, state, |a, b| a > b),
        StoryExpression::LessThan(left, right) => compare_ints(left, right, state, |a, b| a < b),
    }
}

fn evaluate_value(expression: &StoryExpression, state: &StoryRuntimeState) -> Option<StoryValue> {
    match expression {
        StoryExpression::Variable(key) => state.variables.get(key).cloned(),
        StoryExpression::Constant(value) => Some(value.clone()),
        _ => None,
    }
}

fn compare_ints(
    left: &StoryExpression,
    right: &StoryExpression,
    state: &StoryRuntimeState,
    predicate: impl FnOnce(i64, i64) -> bool,
) -> bool {
    match (evaluate_value(left, state), evaluate_value(right, state)) {
        (Some(StoryValue::Int(left)), Some(StoryValue::Int(right))) => predicate(left, right),
        _ => false,
    }
}

fn apply_operations(
    variables: &mut BTreeMap<String, StoryValue>,
    knowledge: &mut BTreeSet<String>,
    operations: &[StoryStateOperation],
) -> Result<(), String> {
    for operation in operations {
        match operation {
            StoryStateOperation::Set { key, value } => {
                variables.insert(key.clone(), value.clone());
            }
            StoryStateOperation::Increment { key, amount } => {
                let Some(StoryValue::Int(value)) = variables.get_mut(key) else {
                    return Err(format!("无法递增非整数状态变量: {key}"));
                };
                *value = value
                    .checked_add(*amount)
                    .ok_or_else(|| format!("状态变量递增溢出: {key}"))?;
            }
            StoryStateOperation::AddKnowledge { key } => {
                knowledge.insert(key.clone());
            }
            StoryStateOperation::RemoveKnowledge { key } => {
                knowledge.remove(key);
            }
        }
    }
    Ok(())
}

fn reject_unmarked_cycles(
    adjacency: &HashMap<&str, Vec<(&str, StoryEdgeKind)>>,
) -> Result<(), String> {
    fn visit<'a>(
        node: &'a str,
        adjacency: &HashMap<&'a str, Vec<(&'a str, StoryEdgeKind)>>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> Result<(), String> {
        if !visited.insert(node) {
            return Ok(());
        }
        visiting.insert(node);
        for (next, kind) in adjacency.get(node).into_iter().flatten() {
            if *kind == StoryEdgeKind::LoopBack {
                continue;
            }
            if visiting.contains(next) {
                return Err(format!("发现未声明 LoopGroup 的循环: {node} -> {next}"));
            }
            visit(next, adjacency, visiting, visited)?;
        }
        visiting.remove(node);
        Ok(())
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for node in adjacency.keys() {
        visit(node, adjacency, &mut visiting, &mut visited)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with_loop() -> StoryGraph {
        StoryGraph {
            schema_version: GALGAME_STORY_GRAPH_SCHEMA_VERSION.into(),
            id: "graph-1".into(),
            project_id: 1,
            source_snapshot_id: 2,
            revision: 1,
            entry_node_id: "start".into(),
            terminal_node_ids: vec!["end".into()],
            nodes: vec![
                node("start", StoryNodeKind::Narrative),
                node("loop-entry", StoryNodeKind::LoopEntry),
                node("body", StoryNodeKind::Narrative),
                node("loop-exit", StoryNodeKind::LoopExit),
                node("end", StoryNodeKind::Terminal),
            ],
            edges: vec![
                edge("e1", "start", "loop-entry", StoryEdgeKind::Sequence),
                edge("e2", "loop-entry", "body", StoryEdgeKind::Sequence),
                edge("loop", "body", "loop-entry", StoryEdgeKind::LoopBack),
                edge("exit", "body", "loop-exit", StoryEdgeKind::Sequence),
                edge("finish", "loop-exit", "end", StoryEdgeKind::Sequence),
            ],
            branch_groups: Vec::new(),
            loop_groups: vec![StoryLoopGroup {
                id: "loop-1".into(),
                entry_node_id: "loop-entry".into(),
                body_exit_node_id: "body".into(),
                back_edge_id: "loop".into(),
                back_to_node_id: "loop-entry".into(),
                exit_node_id: "loop-exit".into(),
                guard: StoryExpression::True,
                state_operations: Vec::new(),
                max_iterations: Some(3),
            }],
            state_variables: Vec::new(),
        }
    }

    fn node(id: &str, kind: StoryNodeKind) -> StoryNode {
        StoryNode {
            id: id.into(),
            kind,
            title: id.into(),
            summary: id.into(),
            primary_source: None,
            source_refs: Vec::new(),
            lines: Vec::new(),
            state_before: BTreeMap::new(),
            state_after: BTreeMap::new(),
            state_operations: Vec::new(),
            media_intents: Vec::new(),
            layout_x: 0.0,
            layout_y: 0.0,
        }
    }

    fn edge(id: &str, from: &str, to: &str, kind: StoryEdgeKind) -> StoryEdge {
        StoryEdge {
            id: id.into(),
            from_node_id: from.into(),
            to_node_id: to.into(),
            from_port: "out".into(),
            to_port: "in".into(),
            kind,
            branch_group_id: None,
            guard: None,
            state_operations: Vec::new(),
            display_order: 0,
        }
    }

    #[test]
    fn explicit_loop_is_valid_and_runtime_can_advance() {
        let graph = graph_with_loop();
        let mut runtime = StoryRuntimeState::new(&graph).unwrap();
        runtime.advance(&graph, "e1").unwrap();
        runtime.advance(&graph, "e2").unwrap();
        runtime.advance(&graph, "loop").unwrap();
        runtime.advance(&graph, "e2").unwrap();
        runtime.advance(&graph, "exit").unwrap();
        runtime.advance(&graph, "finish").unwrap();
        assert_eq!(runtime.current_node_id, "end");
    }

    #[test]
    fn loop_limit_counts_back_edges_and_stops_exactly_at_limit() {
        let graph = graph_with_loop();
        let mut runtime = StoryRuntimeState::new(&graph).unwrap();
        runtime.advance(&graph, "e1").unwrap();
        for _ in 0..3 {
            runtime.advance(&graph, "e2").unwrap();
            runtime.advance(&graph, "loop").unwrap();
        }
        runtime.advance(&graph, "e2").unwrap();
        assert_eq!(runtime.loop_iterations["loop-1"], 3);
        assert!(
            !runtime
                .available_edges(&graph)
                .unwrap()
                .iter()
                .any(|edge| edge.id == "loop")
        );
    }

    #[test]
    fn failed_state_operation_does_not_partially_advance_runtime() {
        let mut graph = graph_with_loop();
        graph.state_variables.push(StoryStateVariable {
            key: "counter".into(),
            value_kind: "int".into(),
            default_value: StoryValue::Int(i64::MAX),
        });
        graph.edges[0]
            .state_operations
            .push(StoryStateOperation::Increment {
                key: "counter".into(),
                amount: 1,
            });
        let mut runtime = StoryRuntimeState::new(&graph).unwrap();
        let before = runtime.clone();
        assert!(runtime.advance(&graph, "e1").is_err());
        assert_eq!(runtime, before);
    }

    #[test]
    fn conditions_reject_type_mismatches() {
        let mut graph = graph_with_loop();
        graph.state_variables.push(StoryStateVariable {
            key: "flag".into(),
            value_kind: "bool".into(),
            default_value: StoryValue::Bool(true),
        });
        graph.edges[0].guard = Some(StoryExpression::GreaterThan(
            Box::new(StoryExpression::Variable("flag".into())),
            Box::new(StoryExpression::Constant(StoryValue::Int(0))),
        ));
        assert!(graph.validate().is_err());
    }

    #[test]
    fn branch_conditions_filter_edges_and_selected_option_updates_state() {
        let mut start = node("start", StoryNodeKind::Narrative);
        start.title = "Start".into();
        let fork = node("fork", StoryNodeKind::Fork);
        let left = node("left", StoryNodeKind::Terminal);
        let right = node("right", StoryNodeKind::Terminal);
        let mut left_edge = edge("left-edge", "fork", "left", StoryEdgeKind::Choice);
        left_edge.branch_group_id = Some("branch".into());
        let mut right_edge = edge("right-edge", "fork", "right", StoryEdgeKind::Choice);
        right_edge.branch_group_id = Some("branch".into());
        let graph = StoryGraph {
            schema_version: GALGAME_STORY_GRAPH_SCHEMA_VERSION.into(),
            id: "branch-graph".into(),
            project_id: 1,
            source_snapshot_id: 2,
            revision: 1,
            entry_node_id: "start".into(),
            terminal_node_ids: vec!["left".into(), "right".into()],
            nodes: vec![start, fork, left, right],
            edges: vec![
                edge("start-edge", "start", "fork", StoryEdgeKind::Sequence),
                left_edge,
                right_edge,
            ],
            branch_groups: vec![StoryBranchGroup {
                id: "branch".into(),
                fork_node_id: "fork".into(),
                options: vec![
                    StoryBranchOption {
                        id: "left-option".into(),
                        label: "Left".into(),
                        edge_id: "left-edge".into(),
                        condition: Some(StoryExpression::Variable("allow_left".into())),
                        state_operations: Vec::new(),
                    },
                    StoryBranchOption {
                        id: "right-option".into(),
                        label: "Right".into(),
                        edge_id: "right-edge".into(),
                        condition: None,
                        state_operations: vec![StoryStateOperation::Set {
                            key: "allow_left".into(),
                            value: StoryValue::Bool(true),
                        }],
                    },
                ],
                join_node_id: None,
            }],
            loop_groups: Vec::new(),
            state_variables: vec![StoryStateVariable {
                key: "allow_left".into(),
                value_kind: "bool".into(),
                default_value: StoryValue::Bool(false),
            }],
        };
        let mut runtime = StoryRuntimeState::new(&graph).unwrap();
        runtime.advance(&graph, "start-edge").unwrap();
        assert_eq!(
            runtime
                .available_edges(&graph)
                .unwrap()
                .iter()
                .map(|edge| edge.id.as_str())
                .collect::<Vec<_>>(),
            vec!["right-edge"]
        );
        runtime.advance(&graph, "right-edge").unwrap();
        assert_eq!(runtime.variables["allow_left"], StoryValue::Bool(true));
    }

    #[test]
    fn unmarked_cycle_is_rejected() {
        let mut graph = graph_with_loop();
        graph.loop_groups.clear();
        assert!(graph.validate().is_err());
    }

    #[test]
    fn ordinary_nodes_cannot_fan_out_without_a_branch_group() {
        let graph = StoryGraph {
            schema_version: GALGAME_STORY_GRAPH_SCHEMA_VERSION.into(),
            id: "untyped-fork".into(),
            project_id: 1,
            source_snapshot_id: 2,
            revision: 1,
            entry_node_id: "start".into(),
            terminal_node_ids: vec!["left".into(), "right".into()],
            nodes: vec![
                node("start", StoryNodeKind::Narrative),
                node("left", StoryNodeKind::Terminal),
                node("right", StoryNodeKind::Terminal),
            ],
            edges: vec![
                edge("left-edge", "start", "left", StoryEdgeKind::Sequence),
                edge("right-edge", "start", "right", StoryEdgeKind::Sequence),
            ],
            branch_groups: Vec::new(),
            loop_groups: Vec::new(),
            state_variables: Vec::new(),
        };

        assert!(graph.validate().unwrap_err().contains("单进单出"));
    }

    #[test]
    fn runtime_rejects_graph_revision_mismatch() {
        let graph = graph_with_loop();
        let mut runtime = StoryRuntimeState::new(&graph).unwrap();
        let mut changed = graph.clone();
        changed.revision = 2;
        assert!(runtime.advance(&changed, "e1").is_err());
    }
}
