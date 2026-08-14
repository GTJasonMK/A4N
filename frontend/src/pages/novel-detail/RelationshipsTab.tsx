import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Background,
  Controls,
  MarkerType,
  Position,
  ReactFlow,
  useNodesState,
  type BezierPathOptions,
  type Edge,
  type Node,
  type NodeChange,
  type ReactFlowInstance,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
  Network,
  Pencil,
  Plus,
  RefreshCcw,
  Trash2,
} from "lucide-react";
import { extractErrorMessage } from "../../api";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { Dropdown } from "../../components/ui/Dropdown";
import { Modal } from "../../components/ui/Modal";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import {
  listProfiles,
  updateCharacterProfile,
  type CharacterProfile,
} from "../../api/character";
import {
  characterAttributeEntries,
  characterAttributeValueText,
} from "../../utils/characterAttributes";
import {
  readPortableItem,
  removePortableItem,
  writePortableItem,
} from "../../utils/portableStorage";
import { EmptyHint } from "./shared";
import { useChapterAnalysisLoader } from "./useNovelDetailLoader";
import { profileRelationshipBaseAttributes } from "../writing-desk/CharacterProfileEditorModal";
import { CharacterDetailDialog } from "../CharacterDetailDialog";

interface CurrentRelationship {
  id: string;
  profile_id: number;
  character_a: string;
  character_b: string;
  description: string;
  last_synced_chapter: number;
  timeline_started: boolean;
  structured: boolean;
  editable: boolean;
}

export const relationshipEditMode = (
  editable: boolean,
  timelineStarted: boolean,
): "initial" | "timeline" | "readonly" => {
  if (editable) return "initial";
  return timelineStarted ? "timeline" : "readonly";
};

interface RelationshipEditorState {
  profileId: number;
  target: string;
  description: string;
  editing: boolean;
}

interface RelationshipNode {
  id: string;
  name: string;
  degree: number;
}

interface RelationshipEdge {
  id: string;
  fromId: string;
  toId: string;
  relationship: CurrentRelationship;
  parallelIndex: number;
}

interface RelationshipGraphData {
  nodes: RelationshipNode[];
  edges: RelationshipEdge[];
  relationshipsByNodeId: Map<string, CurrentRelationship[]>;
}

interface RelationshipNodePosition {
  x: number;
  y: number;
}

type RelationshipNodePositions = Record<string, RelationshipNodePosition>;
type MobileRelationshipView = "graph" | "characters";

interface RelationshipFlowNodeData extends Record<string, unknown> {
  label: React.ReactNode;
  name: string;
  degree: number;
}

interface RelationshipFlowEdgeData extends Record<string, unknown> {
  relationship: CurrentRelationship;
}

type RelationshipFlowNode = Node<RelationshipFlowNodeData, "default">;
type RelationshipFlowEdge = Edge<RelationshipFlowEdgeData, "default"> & {
  pathOptions?: BezierPathOptions;
};

const RELATIONSHIP_NODE_SIZE = 76;
const RELATIONSHIP_LAYOUT_MIN_WIDTH = 720;
const RELATIONSHIP_LAYOUT_MIN_HEIGHT = 480;
const RELATIONSHIP_RING_CAPACITY = 10;
const RELATIONSHIP_RING_GAP = 150;
const RELATIONSHIP_GRAPH_LAYOUT_STORAGE_PREFIX =
  "a4n:relationship-graph-layout:v1";

const syncedChapterText = (chapter: number, timelineStarted: boolean): string =>
  chapter > 0 ? `已同步第 ${chapter} 章` : timelineStarted ? "已有章节记录" : "未同步";

const currentRelationshipsFromProfiles = (
  profiles: CharacterProfile[],
): CurrentRelationship[] =>
  profiles.flatMap((profile) => {
    const characterA = profile.name.trim();
    if (!characterA) return [];
    return characterAttributeEntries(profile, "social")
      .map(([target, value]) => {
        const characterB = target.trim();
        const description = characterAttributeValueText(value);
        if (!characterB || !description) return null;
        return {
          id: `${profile.id}:${characterB}`,
          profile_id: profile.id,
          character_a: characterA,
          character_b: characterB,
          description,
          last_synced_chapter: profile.last_synced_chapter,
          timeline_started: profile.timeline_started,
          structured: typeof value !== "string",
          editable:
            profile.initial_profile_editable && typeof value === "string",
        };
      })
      .filter((item): item is CurrentRelationship => item != null);
  });

export const RelationshipsTab: React.FC<{ projectId: number }> = ({
  projectId,
}) => {
  const { addToast } = useToast();
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [currentDetail, setCurrentDetail] =
    useState<CurrentRelationship | null>(null);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const [editor, setEditor] = useState<RelationshipEditorState | null>(null);
  const [timelineRelationship, setTimelineRelationship] =
    useState<CurrentRelationship | null>(null);
  const [saving, setSaving] = useState(false);
  const activeProjectIdRef = useRef(projectId);
  const mutationSeqRef = useRef(0);
  const mutationInFlightRef = useRef(false);

  activeProjectIdRef.current = projectId;

  const loadRelationships = useCallback(
    async (isCurrent: () => boolean) => {
      try {
        const data = await listProfiles(projectId);
        if (!isCurrent()) return;
        setProfiles(data);
        setLoadedProjectId(projectId);
      } catch (err) {
        if (isCurrent()) {
          setProfiles([]);
          setLoadedProjectId(projectId);
        }
        throw err;
      }
    },
    [projectId],
  );
  const { loading, load } = useChapterAnalysisLoader(
    projectId,
    loadRelationships,
    "加载关系失败",
  );

  useEffect(() => {
    mutationSeqRef.current += 1;
    mutationInFlightRef.current = false;
    setCurrentDetail(null);
    setEditor(null);
    setTimelineRelationship(null);
    setSaving(false);
  }, [projectId]);

  const displayProfiles = loadedProjectId === projectId ? profiles : [];
  const displayLoading = loading || loadedProjectId !== projectId;
  const currentRels = useMemo(
    () => currentRelationshipsFromProfiles(displayProfiles),
    [displayProfiles],
  );
  const editableProfiles = useMemo(
    () => displayProfiles.filter((profile) => profile.initial_profile_editable),
    [displayProfiles],
  );

  useEffect(() => {
    setCurrentDetail((current) => {
      if (!current) return null;
      return (
        currentRels.find((relationship) => relationship.id === current.id) ??
        null
      );
    });
    setTimelineRelationship((current) => {
      if (!current) return null;
      return (
        currentRels.find((relationship) => relationship.id === current.id) ??
        null
      );
    });
  }, [currentRels]);

  const persistInitialRelationship = async (
    profile: CharacterProfile,
    target: string,
    description: string | null,
    successMessage: string,
  ): Promise<boolean> => {
    if (mutationInFlightRef.current) return false;
    mutationInFlightRef.current = true;
    const requestProjectId = projectId;
    const requestSeq = mutationSeqRef.current + 1;
    mutationSeqRef.current = requestSeq;
    setSaving(true);
    try {
      const result = await updateCharacterProfile({
        project_id: requestProjectId,
        profile_id: profile.id,
        expected_current_state: profile.current_state,
        base_attributes: profileRelationshipBaseAttributes(
          profile.current_state,
          target,
          description,
        ),
      });
      if (
        activeProjectIdRef.current !== requestProjectId ||
        mutationSeqRef.current !== requestSeq
      ) {
        return false;
      }
      setProfiles((current) =>
        current.map((item) => (item.id === result.profile.id ? result.profile : item)),
      );
      addToast(
        result.warnings.length > 0
          ? `${successMessage}；${result.warnings.join("；")}`
          : successMessage,
        result.warnings.length > 0 ? "info" : "success",
      );
      await load({ silent: true, force: true });
      return true;
    } catch (error) {
      if (
        activeProjectIdRef.current === requestProjectId &&
        mutationSeqRef.current === requestSeq
      ) {
        addToast(extractErrorMessage(error, "保存初始关系失败"), "error");
      }
      return false;
    } finally {
      if (
        activeProjectIdRef.current === requestProjectId &&
        mutationSeqRef.current === requestSeq
      ) {
        mutationInFlightRef.current = false;
        setSaving(false);
      }
    }
  };

  const openCreate = () => {
    const profile = editableProfiles[0];
    if (!profile) {
      addToast(
        displayProfiles.length === 0
          ? "请先建立至少一个角色档案"
          : displayProfiles.every((item) => item.timeline_started)
            ? "现有角色均已进入章节时间线，请在角色详情中按章节记录关系变化"
            : "当前项目状态不允许直接修改初始关系",
        "info",
      );
      return;
    }
    setEditor({
      profileId: profile.id,
      target: "",
      description: "",
      editing: false,
    });
  };

  const openEdit = (relationship: CurrentRelationship) => {
    const mode = relationshipEditMode(
      relationship.editable,
      relationship.timeline_started,
    );
    if (mode !== "initial") {
      if (mode === "timeline") {
        setCurrentDetail(null);
        setTimelineRelationship(relationship);
      } else {
        addToast(
          relationship.structured
            ? "该关系包含结构化资料，不能用文本表单直接覆盖"
            : "当前项目状态不允许修改初始关系",
          "info",
        );
      }
      return;
    }
    setEditor({
      profileId: relationship.profile_id,
      target: relationship.character_b,
      description: relationship.description,
      editing: true,
    });
  };

  const saveRelationship = async () => {
    if (!editor) return;
    const profile = displayProfiles.find((item) => item.id === editor.profileId);
    const target = editor.target.trim();
    const description = editor.description.trim();
    if (!profile) {
      addToast("关系所属角色已经不存在，请刷新后重试", "error");
      return;
    }
    if (!target || !description) {
      addToast("请填写关系对象和关系描述", "error");
      return;
    }
    if (relationshipNodeId(profile.name) === relationshipNodeId(target)) {
      addToast("关系对象不能与所属角色相同", "error");
      return;
    }
    if (
      !editor.editing &&
      currentRels.some(
        (relationship) =>
          relationship.profile_id === profile.id &&
          relationshipNodeId(relationship.character_b) === relationshipNodeId(target),
      )
    ) {
      addToast("该角色已经存在同名关系，请直接编辑现有关系", "error");
      return;
    }
    if (
      await persistInitialRelationship(
        profile,
        target,
        description,
        editor.editing ? "初始关系已更新" : "初始关系已创建",
      )
    ) {
      setEditor(null);
    }
  };

  const deleteRelationship = async (relationship: CurrentRelationship) => {
    if (!relationship.editable) {
      addToast(
        relationship.timeline_started
          ? "该关系已进入章节时间线，不能直接删除初始值"
          : relationship.structured
            ? "该关系包含结构化资料，不能通过文本关系入口删除"
            : "当前项目状态不允许删除初始关系",
        "info",
      );
      return;
    }
    const confirmed = await confirmDialog({
      title: "删除初始关系",
      message: `删除「${relationship.character_a} → ${relationship.character_b}」的初始关系？`,
      confirmText: "删除",
      dialogType: "danger",
    });
    if (!confirmed) return;
    const profile = displayProfiles.find(
      (item) => item.id === relationship.profile_id,
    );
    if (!profile) {
      addToast("关系所属角色已经不存在，请刷新后重试", "error");
      return;
    }
    if (
      await persistInitialRelationship(
        profile,
        relationship.character_b,
        null,
        "初始关系已删除",
      )
    ) {
      setCurrentDetail(null);
      setEditor(null);
    }
  };

  if (displayLoading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> 加载关系…
      </div>
    );
  }

  const timelineProfile = timelineRelationship
    ? (displayProfiles.find(
        (profile) => profile.id === timelineRelationship.profile_id,
      ) ?? null)
    : null;

  return (
    <div className="content-wide flex h-full min-h-0 flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="text-small leading-relaxed text-book-text-muted">
          正文时间线开始前可直接维护初始关系；同步章节后请在角色详情中按章节记录关系变化。
        </div>
        <BookButton
          variant="primary"
          size="sm"
          onClick={openCreate}
          disabled={saving || editableProfiles.length === 0}
          title={
            editableProfiles.length > 0
              ? "新增初始角色关系"
              : "没有可直接修改初始关系的角色"
          }
        >
          <Plus size={12} /> 新增初始关系
        </BookButton>
      </div>

      <CurrentRelationshipsCard
        projectId={projectId}
        relationships={currentRels}
        onOpen={setCurrentDetail}
      />

      {currentDetail && (
        <CurrentRelationshipDetailDialog
          relationship={currentDetail}
          saving={saving}
          onEdit={() => openEdit(currentDetail)}
          onDelete={() => void deleteRelationship(currentDetail)}
          onClose={() => setCurrentDetail(null)}
        />
      )}

      <RelationshipEditorModal
        state={editor}
        profiles={editableProfiles}
        allProfiles={displayProfiles}
        saving={saving}
        onChange={setEditor}
        onSave={() => void saveRelationship()}
        onClose={() => !saving && setEditor(null)}
      />

      <CharacterDetailDialog
        open={timelineRelationship != null && timelineProfile != null}
        onClose={() => setTimelineRelationship(null)}
        projectId={projectId}
        character={timelineProfile}
        currentChapterNumber={timelineRelationship?.last_synced_chapter}
        initialTab="manage"
        initialEventFilter={
          timelineRelationship
            ? { category: "social", key: timelineRelationship.character_b }
            : null
        }
        onChanged={() => load({ silent: true, force: true })}
      />
    </div>
  );
};

const CurrentRelationshipsCard: React.FC<{
  projectId: number;
  relationships: CurrentRelationship[];
  onOpen: (relationship: CurrentRelationship) => void;
}> = ({ projectId, relationships, onOpen }) => {
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [mobileView, setMobileView] =
    useState<MobileRelationshipView>("graph");
  const [hoveredRelationship, setHoveredRelationship] =
    useState<CurrentRelationship | null>(null);
  const [fixedNodeCount, setFixedNodeCount] = useState(0);
  const [flowInstance, setFlowInstance] = useState<ReactFlowInstance<
    RelationshipFlowNode,
    RelationshipFlowEdge
  > | null>(null);
  const [nodes, setNodes, applyNodesChange] =
    useNodesState<RelationshipFlowNode>([]);
  const graph = useMemo(
    () => buildRelationshipGraph(relationships),
    [relationships],
  );
  const layoutSize = useMemo(
    () => relationshipLayoutSize(graph.nodes.length),
    [graph.nodes.length],
  );
  const edges = useMemo(() => buildRelationshipFlowEdges(graph), [graph]);

  useEffect(() => {
    const stored = loadRelationshipNodePositions(projectId);
    const nodeIds = new Set(graph.nodes.map((node) => node.id));
    const positions = pruneRelationshipNodePositions(stored, nodeIds);
    if (Object.keys(positions).length !== Object.keys(stored).length) {
      saveRelationshipNodePositions(projectId, positions);
    }
    setFixedNodeCount(Object.keys(positions).length);
    setNodes(buildRelationshipFlowNodes(graph, positions, layoutSize));
    setSelectedNodeId((current) =>
      current && nodeIds.has(current) ? current : null,
    );
    const frame = window.requestAnimationFrame(() => {
      void flowInstance?.fitView({ padding: 0.22, maxZoom: 1.15 });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [flowInstance, graph, layoutSize, projectId, setNodes]);

  const flowNodes = useMemo(
    () =>
      nodes.map((node) => {
        const selected = node.id === selectedNodeId;
        return node.selected === selected ? node : { ...node, selected };
      }),
    [nodes, selectedNodeId],
  );
  const selectedNode = selectedNodeId
    ? (graph.nodes.find((node) => node.id === selectedNodeId) ?? null)
    : null;
  const selectedRelationships = selectedNodeId
    ? (graph.relationshipsByNodeId.get(selectedNodeId) ?? [])
    : [];

  const handleNodesChange = useCallback(
    (changes: NodeChange<RelationshipFlowNode>[]) => {
      applyNodesChange(
        changes.filter(
          (change) =>
            change.type === "position" || change.type === "dimensions",
        ),
      );
    },
    [applyNodesChange],
  );

  const persistDraggedNode = useCallback(
    (_event: MouseEvent | TouchEvent, node: RelationshipFlowNode) => {
      const positions = pruneRelationshipNodePositions(
        loadRelationshipNodePositions(projectId),
        new Set(graph.nodes.map((item) => item.id)),
      );
      positions[node.id] = relationshipNodePosition(node.position, layoutSize);
      saveRelationshipNodePositions(projectId, positions);
      setFixedNodeCount(Object.keys(positions).length);
    },
    [graph.nodes, layoutSize, projectId],
  );

  const resetNodeLayout = useCallback(() => {
    clearRelationshipNodePositions(projectId);
    setFixedNodeCount(0);
    setNodes(buildRelationshipFlowNodes(graph, {}, layoutSize));
    setSelectedNodeId(null);
    setHoveredRelationship(null);
    window.requestAnimationFrame(() => {
      void flowInstance?.fitView({ padding: 0.22, maxZoom: 1.15 });
    });
  }, [flowInstance, graph, layoutSize, projectId, setNodes]);

  if (relationships.length === 0) {
    return (
      <div className="a4n-flat-surface p-5">
        <EmptyHint text="角色档案中还没有社会关系。" />
      </div>
    );
  }

  return (
    <section className="a4n-flat-surface flex min-h-0 flex-col overflow-hidden p-0 lg:flex-1">
      <div className="flex min-h-12 items-center border-b border-book-border/60 bg-book-bg-paper px-3 py-2">
        <div className="w-full lg:hidden">
          <SegmentedControl
            ariaLabel="关系页视图"
            value={mobileView}
            onChange={setMobileView}
            options={[
              { id: "graph", label: "关系图" },
              { id: "characters", label: "角色列表" },
            ]}
          />
        </div>
        <div className="hidden min-w-0 items-baseline gap-2 lg:flex">
          <div className="text-body font-semibold text-book-text-main">
            关系图
          </div>
          <div className="text-small text-book-text-muted">
            {graph.nodes.length} 位角色 · {graph.edges.length} 条关系
          </div>
        </div>
      </div>

      <div className="grid min-h-0 grid-cols-1 lg:flex-1 lg:grid-cols-[minmax(0,1fr)_288px]">
        <div
          className={
            (mobileView === "graph" ? "block" : "hidden") +
            " relative h-[360px] min-h-0 min-w-0 bg-book-bg sm:h-[420px] lg:block lg:h-auto"
          }
        >
          {fixedNodeCount > 0 ? (
            <div className="absolute right-3 top-3 z-10">
              <Dropdown
                items={[
                  {
                    label: "恢复自动布局",
                    icon: <RefreshCcw size={13} />,
                    onClick: resetNodeLayout,
                  },
                ]}
              />
            </div>
          ) : null}
          <ReactFlow<RelationshipFlowNode, RelationshipFlowEdge>
            nodes={flowNodes}
            edges={edges}
            onNodesChange={handleNodesChange}
            onNodeClick={(_event, node) => {
              setSelectedNodeId((current) =>
                current === node.id ? null : node.id,
              );
              setMobileView("characters");
            }}
            onNodeDragStop={persistDraggedNode}
            onPaneClick={() => setSelectedNodeId(null)}
            onEdgeClick={(_event, edge) => {
              if (edge.data) onOpen(edge.data.relationship);
            }}
            onEdgeMouseEnter={(_event, edge) =>
              setHoveredRelationship(edge.data?.relationship ?? null)
            }
            onEdgeMouseLeave={() => setHoveredRelationship(null)}
            onInit={setFlowInstance}
            nodesConnectable={false}
            edgesReconnectable={false}
            deleteKeyCode={null}
            minZoom={0.45}
            maxZoom={2}
            fitView
            fitViewOptions={{ padding: 0.22, maxZoom: 1.15 }}
            nodeExtent={[
              [0, 0],
              [
                layoutSize.width - RELATIONSHIP_NODE_SIZE,
                layoutSize.height - RELATIONSHIP_NODE_SIZE,
              ],
            ]}
            className="a4n-relationship-flow"
            aria-label="当前角色关系网"
          >
            <Controls
              showInteractive={false}
              className="a4n-flow-controls"
            />
            <Background
              color="rgb(var(--color-text-tertiary) / 0.2)"
              gap={28}
              size={1}
            />
          </ReactFlow>
        </div>

        <RelationshipGraphPanel
          graph={graph}
          hoveredRelationship={hoveredRelationship}
          selectedNode={selectedNode}
          selectedRelationships={selectedRelationships}
          selectedNodeId={selectedNodeId}
          mobileVisible={mobileView === "characters"}
          onSelectNode={setSelectedNodeId}
          onOpen={onOpen}
        />
      </div>
    </section>
  );
};

const RelationshipGraphPanel: React.FC<{
  graph: RelationshipGraphData;
  hoveredRelationship: CurrentRelationship | null;
  selectedNode: RelationshipNode | null;
  selectedRelationships: CurrentRelationship[];
  selectedNodeId: string | null;
  mobileVisible: boolean;
  onSelectNode: (nodeId: string | null) => void;
  onOpen: (relationship: CurrentRelationship) => void;
}> = ({
  graph,
  hoveredRelationship,
  selectedNode,
  selectedRelationships,
  selectedNodeId,
  mobileVisible,
  onSelectNode,
  onOpen,
}) => {
  const prominentNodes = [...graph.nodes].sort(
    (a, b) => b.degree - a.degree || a.name.localeCompare(b.name),
  );

  return (
    <aside
      className={`${mobileVisible ? "block" : "hidden"} h-[440px] min-h-0 min-w-0 overflow-y-auto border-t border-book-border bg-book-bg-paper custom-scrollbar sm:h-[520px] lg:block lg:h-auto lg:border-l lg:border-t-0`}
    >
      {hoveredRelationship ? (
        <section className="border-b border-book-primary/15 bg-book-primary/5 px-4 py-3">
          <div className="mb-1 flex min-w-0 items-center gap-1.5 text-small font-semibold text-book-text-main">
            <span className="truncate">{hoveredRelationship.character_a}</span>
            <span className="text-book-primary">→</span>
            <span className="truncate">{hoveredRelationship.character_b}</span>
          </div>
          <div className="line-clamp-2 text-small leading-relaxed text-book-text-muted">
            {hoveredRelationship.description}
          </div>
        </section>
      ) : null}

      {selectedNode ? (
        <section className="border-b border-book-border/70">
          <div className="flex items-center gap-2.5 px-4 py-3.5">
            <InitialBadge value={selectedNode.name} />
            <div className="min-w-0 flex-1">
              <div
                className="truncate text-sm font-semibold text-book-text-main"
                title={selectedNode.name}
              >
                {selectedNode.name}
              </div>
              <div className="mt-0.5 text-small text-book-text-muted">
                共 {selectedNode.degree} 条关系
              </div>
            </div>
          </div>

          <div className="grid gap-1.5 border-t border-book-border/55 p-2">
            {selectedRelationships.map((relationship) => (
              <button
                key={relationship.id}
                type="button"
                onClick={() => onOpen(relationship)}
                className="block w-full rounded-xl border border-book-border/40 bg-book-bg-paper/45 px-3 py-2.5 text-left transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/55 focus-visible:border-book-primary/45 focus-visible:outline-none"
                title="查看关系详情"
              >
                <div className="flex min-w-0 items-center gap-1.5 text-small font-semibold text-book-text-main">
                  <span className="truncate">{relationship.character_a}</span>
                  <span className="text-book-primary">→</span>
                  <span className="truncate">{relationship.character_b}</span>
                </div>
                <div className="mt-0.5 line-clamp-2 text-small leading-relaxed text-book-text-muted">
                  {relationship.description}
                </div>
              </button>
            ))}
          </div>
        </section>
      ) : null}

      <section className="px-2 py-3">
        <div className="mb-1 px-2 text-small font-semibold text-book-text-main">
          全部角色
        </div>
        <div className="grid gap-1.5">
          {prominentNodes.map((node) => {
            const selected = node.id === selectedNodeId;
            return (
              <button
                key={node.id}
                type="button"
                onClick={() => onSelectNode(selected ? null : node.id)}
                aria-pressed={selected}
                className={`grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 rounded-xl border px-2 py-2 text-left transition-colors ${
                  selected
                    ? "border-book-primary/50 bg-book-primary/10 text-book-primary"
                    : "border-book-border/35 bg-book-bg-paper/40 text-book-text-muted hover:border-book-primary/30 hover:bg-book-bg-tertiary/65 hover:text-book-text-main"
                }`}
                title={node.name}
              >
                <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-book-bg-tertiary text-caption font-semibold text-book-text-muted">
                  {initialOf(node.name)}
                </span>
                <span className="min-w-0 truncate text-xs font-medium">
                  {node.name}
                </span>
                <span className="shrink-0 text-caption text-book-text-muted">
                  {node.degree} 条
                </span>
              </button>
            );
          })}
        </div>
      </section>
    </aside>
  );
};

const InitialBadge: React.FC<{ value: string }> = ({ value }) => (
  <div className="flex h-9 w-9 shrink-0 select-none items-center justify-center rounded-full bg-book-primary/10 text-xs font-semibold text-book-primary">
    {initialOf(value)}
  </div>
);

const RelationshipEditorModal: React.FC<{
  state: RelationshipEditorState | null;
  profiles: CharacterProfile[];
  allProfiles: CharacterProfile[];
  saving: boolean;
  onChange: (state: RelationshipEditorState | null) => void;
  onSave: () => void;
  onClose: () => void;
}> = ({ state, profiles, allProfiles, saving, onChange, onSave, onClose }) => {
  if (!state) return null;
  const profile = profiles.find((item) => item.id === state.profileId);
  const targetOptions = allProfiles.filter((item) => item.id !== state.profileId);

  return (
    <Modal
      isOpen
      onClose={onClose}
      title={state.editing ? "编辑初始关系" : "新增初始关系"}
      subtitle="直接更新正式角色档案，不创建虚假的章节事件"
      icon={state.editing ? <Pencil size={15} /> : <Plus size={15} />}
      maxWidthClassName="max-w-lg"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!saving}
      closeOnEscape={!saving}
      showCloseButton={!saving}
      footer={
        <BookButton
          variant="primary"
          onClick={onSave}
          disabled={
            saving ||
            !profile ||
            !state.target.trim() ||
            !state.description.trim()
          }
        >
          {saving ? (
            <span className="inline-flex items-center gap-2">
              <LoadingSpinner size="sm" /> 保存中…
            </span>
          ) : state.editing ? (
            "保存修改"
          ) : (
            "保存"
          )}
        </BookButton>
      }
    >
      <div className="grid gap-3">
        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-book-text-sub">所属角色 *</span>
          <select
            value={state.profileId}
            onChange={(event) =>
              onChange({
                ...state,
                profileId: Number(event.target.value),
              })
            }
            disabled={saving || state.editing}
            className="pyqt-input"
          >
            {profiles.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </select>
        </label>

        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-book-text-sub">关系对象 *</span>
          <input
            type="text"
            list="initial-relationship-targets"
            value={state.target}
            onChange={(event) => onChange({ ...state, target: event.target.value })}
            disabled={saving || state.editing}
            className="pyqt-input"
            placeholder="角色、组织或阵营名称"
          />
          <datalist id="initial-relationship-targets">
            {targetOptions.map((item) => (
              <option key={item.id} value={item.name} />
            ))}
          </datalist>
        </label>

        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-book-text-sub">关系描述 *</span>
          <textarea
            value={state.description}
            onChange={(event) =>
              onChange({ ...state, description: event.target.value })
            }
            disabled={saving}
            className="pyqt-input min-h-28 resize-y"
            placeholder="例如：旧友，彼此信任但对过去有所隐瞒"
          />
        </label>
      </div>
    </Modal>
  );
};

const CurrentRelationshipDetailDialog: React.FC<{
  relationship: CurrentRelationship;
  saving: boolean;
  onEdit: () => void;
  onDelete: () => void;
  onClose: () => void;
}> = ({ relationship, saving, onEdit, onDelete, onClose }) => {
  const editMode = relationshipEditMode(
    relationship.editable,
    relationship.timeline_started,
  );
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={`${relationship.character_a} / ${relationship.character_b}`}
      subtitle={`社会关系 · ${syncedChapterText(
        relationship.last_synced_chapter,
        relationship.timeline_started,
      )}`}
      icon={<Network size={15} />}
      maxWidthClassName="max-w-3xl"
      bodyClassName="px-4 py-4"
      footer={
        editMode === "initial" ? (
          <div className="flex items-center gap-2">
            <BookButton variant="secondary" onClick={onEdit} disabled={saving}>
              <Pencil size={12} /> 编辑
            </BookButton>
            <BookButton variant="danger" onClick={onDelete} disabled={saving}>
              <Trash2 size={12} /> 删除
            </BookButton>
          </div>
        ) : editMode === "timeline" ? (
          <BookButton variant="primary" onClick={onEdit} disabled={saving}>
            <Pencil size={12} /> 记录关系变化
          </BookButton>
        ) : undefined
      }
    >
      <div>
        <div className="mb-1 text-xs font-semibold text-book-text-main">
          关系描述
        </div>
        <div className="whitespace-pre-wrap text-sm leading-7 text-book-text-sub">
          {relationship.description}
        </div>
        {editMode !== "initial" ? (
          <div className="mt-4 rounded-xl border border-book-border/60 bg-book-bg-tertiary/45 px-3 py-2 text-small leading-relaxed text-book-text-muted">
            {relationship.timeline_started
              ? "该角色已经进入章节时间线。后续关系变化应在角色详情中按实际章节记录，不能直接重写初始关系。"
              : relationship.structured
                ? "该关系包含结构化资料，文本表单不会将其压平成普通描述。"
                : "当前项目状态不允许直接重写初始关系。"}
          </div>
        ) : null}
      </div>
    </Modal>
  );
};

function buildRelationshipGraph(
  relationships: CurrentRelationship[],
): RelationshipGraphData {
  const nodeById = new Map<string, RelationshipNode>();
  const relationshipsByNodeId = new Map<string, CurrentRelationship[]>();
  const ensureNode = (name: string): RelationshipNode => {
    const id = relationshipNodeId(name);
    const existing = nodeById.get(id);
    if (existing) return existing;
    const node: RelationshipNode = {
      id,
      name: name.trim(),
      degree: 0,
    };
    nodeById.set(id, node);
    relationshipsByNodeId.set(id, []);
    return node;
  };

  const edges = relationships.map((relationship) => {
    const from = ensureNode(relationship.character_a);
    const to = ensureNode(relationship.character_b);
    from.degree += 1;
    to.degree += 1;
    relationshipsByNodeId.get(from.id)?.push(relationship);
    if (from.id !== to.id) {
      relationshipsByNodeId.get(to.id)?.push(relationship);
    }
    return {
      id: relationship.id,
      fromId: from.id,
      toId: to.id,
      relationship,
      parallelIndex: 0,
    };
  });

  const parallelGroups = new Map<string, RelationshipEdge[]>();
  edges.forEach((edge) => {
    const key = [edge.fromId, edge.toId].sort().join("::");
    const group = parallelGroups.get(key) ?? [];
    group.push(edge);
    parallelGroups.set(key, group);
  });
  parallelGroups.forEach((group) => {
    group.forEach((edge, index) => {
      edge.parallelIndex = index;
    });
  });

  return {
    nodes: [...nodeById.values()],
    edges,
    relationshipsByNodeId,
  };
}

function relationshipLayoutSize(nodeCount: number): {
  width: number;
  height: number;
} {
  const ringCount = Math.max(
    1,
    Math.ceil(Math.max(0, nodeCount - 1) / RELATIONSHIP_RING_CAPACITY),
  );
  const diameter =
    ringCount * RELATIONSHIP_RING_GAP * 2 + RELATIONSHIP_NODE_SIZE * 2;
  return {
    width: Math.max(RELATIONSHIP_LAYOUT_MIN_WIDTH, diameter),
    height: Math.max(RELATIONSHIP_LAYOUT_MIN_HEIGHT, diameter),
  };
}

function automaticRelationshipNodeCenter(
  index: number,
  nodeCount: number,
  layout: { width: number; height: number },
): RelationshipNodePosition {
  if (nodeCount <= 1 || index === 0) {
    return { x: layout.width / 2, y: layout.height / 2 };
  }
  const ring = Math.floor((index - 1) / RELATIONSHIP_RING_CAPACITY) + 1;
  const ringStart = 1 + (ring - 1) * RELATIONSHIP_RING_CAPACITY;
  const ringCount = Math.min(
    RELATIONSHIP_RING_CAPACITY,
    nodeCount - ringStart,
  );
  const ringIndex = index - ringStart;
  const angle =
    (Math.PI * 2 * ringIndex) / Math.max(1, ringCount) - Math.PI / 2;
  const radius = ring * RELATIONSHIP_RING_GAP;
  return {
    x: layout.width / 2 + Math.cos(angle) * radius,
    y: layout.height / 2 + Math.sin(angle) * radius,
  };
}

function buildRelationshipFlowNodes(
  graph: RelationshipGraphData,
  positions: RelationshipNodePositions,
  layout: { width: number; height: number },
): RelationshipFlowNode[] {
  const ordered = [...graph.nodes].sort(
    (left, right) =>
      right.degree - left.degree || left.name.localeCompare(right.name),
  );
  return ordered.map((node, index) => {
    const saved = positions[node.id];
    const center = saved
      ? { x: saved.x * layout.width, y: saved.y * layout.height }
      : automaticRelationshipNodeCenter(index, ordered.length, layout);
    const halfNode = RELATIONSHIP_NODE_SIZE / 2;
    return {
      id: node.id,
      type: "default",
      position: {
        x: center.x - halfNode,
        y: center.y - halfNode,
      },
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
      data: {
        name: node.name,
        degree: node.degree,
        label: (
          <div className="flex h-full min-w-0 flex-col items-center justify-center px-1 text-center">
            <span className="text-sm font-bold text-book-primary">
              {initialOf(node.name)}
            </span>
            <span className="mt-0.5 line-clamp-2 max-w-full text-caption font-semibold leading-tight text-book-text-main">
              {node.name}
            </span>
            <span className="mt-0.5 text-[9px] text-book-text-muted">
              {node.degree} 条
            </span>
          </div>
        ),
      },
      style: {
        width: RELATIONSHIP_NODE_SIZE,
        height: RELATIONSHIP_NODE_SIZE,
      },
      className: "a4n-relationship-node",
      draggable: true,
      selectable: true,
      deletable: false,
      connectable: false,
      ariaLabel: node.name + "，" + node.degree + " 条关系",
    };
  });
}

function buildRelationshipFlowEdges(
  graph: RelationshipGraphData,
): RelationshipFlowEdge[] {
  const showLabels = graph.edges.length <= 8;
  return graph.edges.map((edge) => ({
    id: edge.id,
    type: "default",
    source: edge.fromId,
    target: edge.toId,
    data: { relationship: edge.relationship },
    pathOptions: {
      curvature: Math.min(0.85, 0.2 + edge.parallelIndex * 0.18),
    },
    label: showLabels
      ? relationshipEdgeSummary(edge.relationship.description)
      : undefined,
    markerEnd: {
      type: MarkerType.ArrowClosed,
      color: "rgb(var(--color-text-secondary))",
    },
    interactionWidth: 20,
    className: "a4n-relationship-edge",
    selectable: true,
    deletable: false,
    focusable: true,
    ariaLabel:
      edge.relationship.character_a +
      " 到 " +
      edge.relationship.character_b +
      "：" +
      edge.relationship.description,
  }));
}

function relationshipNodePosition(
  position: { x: number; y: number },
  layout: { width: number; height: number },
): RelationshipNodePosition {
  return {
    x: (position.x + RELATIONSHIP_NODE_SIZE / 2) / layout.width,
    y: (position.y + RELATIONSHIP_NODE_SIZE / 2) / layout.height,
  };
}

function relationshipLayoutStorageKey(projectId: number): string {
  return RELATIONSHIP_GRAPH_LAYOUT_STORAGE_PREFIX + ":" + projectId;
}

function loadRelationshipNodePositions(
  projectId: number,
): RelationshipNodePositions {
  const raw = readPortableItem(relationshipLayoutStorageKey(projectId));
  if (!raw) return {};
  const value = JSON.parse(raw) as unknown;
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("关系图布局必须是对象");
  }
  const positions = (value as Record<string, unknown>).positions;
  if (!positions || typeof positions !== "object" || Array.isArray(positions)) {
    throw new Error("关系图布局缺少 positions 对象");
  }

  const parsed: RelationshipNodePositions = {};
  for (const [nodeId, rawPosition] of Object.entries(positions)) {
    if (!nodeId.trim()) {
      throw new Error("关系图布局包含空节点 ID");
    }
    if (
      !rawPosition ||
      typeof rawPosition !== "object" ||
      Array.isArray(rawPosition)
    ) {
      throw new Error("关系图节点位置无效: " + nodeId);
    }
    const record = rawPosition as Record<string, unknown>;
    if (
      typeof record.x !== "number" ||
      !Number.isFinite(record.x) ||
      record.x < 0 ||
      record.x > 1 ||
      typeof record.y !== "number" ||
      !Number.isFinite(record.y) ||
      record.y < 0 ||
      record.y > 1
    ) {
      throw new Error("关系图节点坐标无效: " + nodeId);
    }
    parsed[nodeId] = { x: record.x, y: record.y };
  }
  return parsed;
}

function saveRelationshipNodePositions(
  projectId: number,
  positions: RelationshipNodePositions,
) {
  if (Object.keys(positions).length === 0) {
    clearRelationshipNodePositions(projectId);
    return;
  }
  const saved = writePortableItem(
    relationshipLayoutStorageKey(projectId),
    JSON.stringify({ positions }),
  );
  if (!saved) {
    throw new Error("关系图布局存储键无效");
  }
}

function clearRelationshipNodePositions(projectId: number) {
  removePortableItem(relationshipLayoutStorageKey(projectId));
}

function pruneRelationshipNodePositions(
  positions: RelationshipNodePositions,
  nodeIds: Set<string>,
): RelationshipNodePositions {
  return Object.fromEntries(
    Object.entries(positions).filter(([nodeId]) => nodeIds.has(nodeId)),
  );
}

function relationshipNodeId(name: string): string {
  return name.trim().toLocaleLowerCase();
}

function initialOf(value: string): string {
  return (value.trim().charAt(0) || "?").toUpperCase();
}

function relationshipEdgeSummary(description: string): string {
  const normalized = description.replace(/\s+/g, " ").trim();
  if (!normalized) return "关系";
  const splitIndex = normalized.search(/[。；;，,]/);
  return splitIndex > 0 ? normalized.slice(0, splitIndex) : normalized;
}
