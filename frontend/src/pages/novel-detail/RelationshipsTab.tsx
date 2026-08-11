import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Network,
  Pencil,
  Plus,
  RefreshCcw,
  RotateCcw,
  Trash2,
  ZoomIn,
  ZoomOut, } from "lucide-react";
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
import { readCssRgbVariable, rgba } from "../../utils/cssColor";
import {
  readPortableItem,
  removePortableItem,
  writePortableItem,
} from "../../utils/portableStorage";
import { EmptyHint } from "./shared";
import { useChapterAnalysisLoader } from "./useNovelDetailLoader";
import { THEME_APPLIED_EVENT } from "../../theme/activeTheme";
import { profileRelationshipBaseAttributes } from "../writing-desk/CharacterProfileEditorModal";
import { CharacterDetailDialog } from "../CharacterDetailDialog";

interface CurrentRelationship {
  id: string;
  profile_id: number;
  character_a: string;
  character_b: string;
  description: string;
  last_synced_chapter: number;
  editable: boolean;
}

export const relationshipEditMode = (
  editable: boolean,
  lastSyncedChapter: number,
): "initial" | "timeline" | "readonly" => {
  if (editable) return "initial";
  return lastSyncedChapter > 0 ? "timeline" : "readonly";
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
  profileId: number | null;
  degree: number;
}

interface PositionedRelationshipNode extends RelationshipNode {
  x: number;
  y: number;
  radius: number;
  colorIndex: number;
}

interface RelationshipEdge {
  id: string;
  fromId: string;
  toId: string;
  relationship: CurrentRelationship;
  parallelIndex: number;
  parallelCount: number;
}

interface RelationshipGraphData {
  nodes: RelationshipNode[];
  edges: RelationshipEdge[];
  relationshipsByNodeId: Map<string, CurrentRelationship[]>;
}

interface RelationshipGraphLayout {
  width: number;
  height: number;
  nodes: PositionedRelationshipNode[];
  edges: RelationshipEdge[];
  nodeById: Map<string, PositionedRelationshipNode>;
  relationshipsByNodeId: Map<string, CurrentRelationship[]>;
}

type RelationshipGraphHit =
  { kind: "node"; id: string } | { kind: "edge"; id: string };

interface RelationshipGraphViewport {
  scale: number;
  offsetX: number;
  offsetY: number;
}

interface RelationshipGraphPanDrag {
  pointerId: number;
  startX: number;
  startY: number;
  baseOffsetX: number;
  baseOffsetY: number;
  moved: boolean;
}

interface RelationshipGraphNodeDrag {
  pointerId: number;
  nodeId: string;
  startGraphX: number;
  startGraphY: number;
  nodeStartX: number;
  nodeStartY: number;
  moved: boolean;
}

type RelationshipGraphDrag =
  | ({ kind: "pan" } & RelationshipGraphPanDrag)
  | ({ kind: "node" } & RelationshipGraphNodeDrag);

interface RelationshipEdgeGeometry {
  start: { x: number; y: number };
  end: { x: number; y: number };
  control: { x: number; y: number };
  arrow: { x: number; y: number; angle: number };
  selfLoop: boolean;
  loopRadius?: number;
  loopStartAngle?: number;
  loopEndAngle?: number;
}

interface CanvasSize {
  width: number;
  height: number;
}

interface RelationshipNodePosition {
  x: number;
  y: number;
}

type RelationshipNodePositions = Record<string, RelationshipNodePosition>;
type MobileRelationshipView = "graph" | "characters";

const DEFAULT_CANVAS_SIZE: CanvasSize = { width: 720, height: 480 };
const MIN_CANVAS_WIDTH = 360;
const MIN_CANVAS_HEIGHT = 360;
const RELATIONSHIP_NODE_COLOR_COUNT = 5;
const DEFAULT_GRAPH_VIEWPORT: RelationshipGraphViewport = {
  scale: 1,
  offsetX: 0,
  offsetY: 0,
};
const MIN_GRAPH_SCALE = 0.8;
const MAX_GRAPH_SCALE = 2.4;
const RELATIONSHIP_GRAPH_LAYOUT_STORAGE_PREFIX =
  "a4n:relationship-graph-layout:v1";

const syncedChapterText = (chapter: number): string =>
  chapter > 0 ? `已同步第 ${chapter} 章` : "未同步";

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
          editable:
            profile.last_synced_chapter <= 0 && typeof value === "string",
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
    () => displayProfiles.filter((profile) => profile.last_synced_chapter <= 0),
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
          : "现有角色均已进入章节时间线，请在角色详情中按章节记录关系变化",
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
      relationship.last_synced_chapter,
    );
    if (mode !== "initial") {
      if (mode === "timeline") {
        setCurrentDetail(null);
        setTimelineRelationship(relationship);
      } else {
        addToast("该关系包含结构化资料，不能用文本表单直接覆盖", "info");
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
        relationship.last_synced_chapter > 0
          ? "该关系已进入章节时间线，不能直接删除初始值"
          : "该关系包含结构化资料，不能通过文本关系入口删除",
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
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const canvasHostRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<RelationshipGraphDrag | null>(null);
  const suppressClickRef = useRef(false);
  const nodePositionsRef = useRef<RelationshipNodePositions>({});
  const [canvasSize, setCanvasSize] = useState<CanvasSize>(DEFAULT_CANVAS_SIZE);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [mobileView, setMobileView] =
    useState<MobileRelationshipView>("graph");
  const [hoverTarget, setHoverTarget] = useState<RelationshipGraphHit | null>(
    null,
  );
  const [viewport, setViewport] = useState<RelationshipGraphViewport>(
    DEFAULT_GRAPH_VIEWPORT,
  );
  const [nodePositions, setNodePositions] = useState<RelationshipNodePositions>(
    () => loadRelationshipNodePositions(projectId),
  );
  const [themeVersion, setThemeVersion] = useState(0);
  const graph = useMemo(
    () => buildRelationshipGraph(relationships),
    [relationships],
  );
  const layout = useMemo(
    () => layoutRelationshipGraph(graph, canvasSize, nodePositions),
    [canvasSize, graph, nodePositions],
  );
  const fixedNodeCount = Object.keys(nodePositions).length;

  useLayoutEffect(() => {
    const host = canvasHostRef.current;
    if (!host) return undefined;
    const update = () => {
      const rect = host.getBoundingClientRect();
      setCanvasSize({
        width: Math.max(MIN_CANVAS_WIDTH, Math.round(rect.width)),
        height: Math.max(MIN_CANVAS_HEIGHT, Math.round(rect.height)),
      });
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(host);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const handleThemeApplied = () => setThemeVersion((version) => version + 1);
    window.addEventListener(THEME_APPLIED_EVENT, handleThemeApplied);
    return () =>
      window.removeEventListener(THEME_APPLIED_EVENT, handleThemeApplied);
  }, []);

  useEffect(() => {
    const stored = loadRelationshipNodePositions(projectId);
    nodePositionsRef.current = stored;
    setNodePositions(stored);
    setViewport(DEFAULT_GRAPH_VIEWPORT);
  }, [projectId]);

  useEffect(() => {
    nodePositionsRef.current = nodePositions;
  }, [nodePositions]);

  useEffect(() => {
    setSelectedNodeId((current) => {
      if (!current) return null;
      return layout.nodeById.has(current) ? current : null;
    });
  }, [layout.nodeById]);

  useEffect(() => {
    setViewport(DEFAULT_GRAPH_VIEWPORT);
  }, [graph]);

  useEffect(() => {
    setNodePositions((current) => {
      const pruned = pruneRelationshipNodePositions(current, layout.nodeById);
      if (sameRelationshipNodePositions(current, pruned)) return current;
      nodePositionsRef.current = pruned;
      saveRelationshipNodePositions(projectId, pruned);
      return pruned;
    });
  }, [layout.nodeById, projectId]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    drawRelationshipGraph(
      canvas,
      layout,
      selectedNodeId,
      hoverTarget,
      viewport,
    );
  }, [hoverTarget, layout, selectedNodeId, themeVersion, viewport]);

  const selectedNode = selectedNodeId
    ? (layout.nodeById.get(selectedNodeId) ?? null)
    : null;
  const selectedRelationships = selectedNodeId
    ? (layout.relationshipsByNodeId.get(selectedNodeId) ?? [])
    : [];
  const hoveredRelationship =
    hoverTarget?.kind === "edge"
      ? (layout.edges.find((edge) => edge.id === hoverTarget.id)
          ?.relationship ?? null)
      : null;

  const handlePointerMove = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const drag = dragRef.current;
    if (drag?.kind === "node" && drag.pointerId === event.pointerId) {
      const point = canvasPoint(event, layout, viewport);
      if (!point) return;
      const deltaX = point.x - drag.startGraphX;
      const deltaY = point.y - drag.startGraphY;
      const moved = drag.moved || Math.hypot(deltaX, deltaY) > 3;
      dragRef.current = { ...drag, moved };
      const next = clampRelationshipNodePosition(
        drag.nodeStartX + deltaX,
        drag.nodeStartY + deltaY,
        layout,
        drag.nodeId,
      );
      setNodePositions((current) => {
        const nextPositions = {
          ...current,
          [drag.nodeId]: {
            x: next.x / layout.width,
            y: next.y / layout.height,
          },
        };
        nodePositionsRef.current = nextPositions;
        return nextPositions;
      });
      setHoverTarget({ kind: "node", id: drag.nodeId });
      if (canvasRef.current) canvasRef.current.style.cursor = "grabbing";
      return;
    }

    if (drag?.kind === "pan" && drag.pointerId === event.pointerId) {
      const deltaX = event.clientX - drag.startX;
      const deltaY = event.clientY - drag.startY;
      const moved = drag.moved || Math.hypot(deltaX, deltaY) > 3;
      dragRef.current = { ...drag, moved };
      setViewport((current) =>
        clampGraphViewport(
          {
            ...current,
            offsetX: drag.baseOffsetX + deltaX,
            offsetY: drag.baseOffsetY + deltaY,
          },
          layout,
        ),
      );
      setHoverTarget(null);
      if (canvasRef.current) canvasRef.current.style.cursor = "grabbing";
      return;
    }

    const point = canvasPoint(event, layout, viewport);
    const hit = point
      ? hitTestRelationshipGraph(layout, point.x, point.y)
      : null;
    setHoverTarget((current) => (sameHit(current, hit) ? current : hit));
    if (canvasRef.current)
      canvasRef.current.style.cursor = relationshipGraphCursor(hit);
  };

  const handlePointerLeave = () => {
    if (!dragRef.current) {
      setHoverTarget(null);
      if (canvasRef.current) canvasRef.current.style.cursor = "default";
    }
  };

  const handlePointerDown = (event: React.PointerEvent<HTMLCanvasElement>) => {
    if (event.button !== 0) return;
    const point = canvasPoint(event, layout, viewport);
    const hit = point
      ? hitTestRelationshipGraph(layout, point.x, point.y)
      : null;
    if (hit?.kind === "node" && point) {
      const node = layout.nodeById.get(hit.id);
      if (!node) return;
      dragRef.current = {
        kind: "node",
        pointerId: event.pointerId,
        nodeId: hit.id,
        startGraphX: point.x,
        startGraphY: point.y,
        nodeStartX: node.x,
        nodeStartY: node.y,
        moved: false,
      };
      event.currentTarget.setPointerCapture(event.pointerId);
      if (canvasRef.current) canvasRef.current.style.cursor = "grabbing";
      return;
    }
    if (hit) return;
    dragRef.current = {
      kind: "pan",
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      baseOffsetX: viewport.offsetX,
      baseOffsetY: viewport.offsetY,
      moved: false,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    if (canvasRef.current) canvasRef.current.style.cursor = "grabbing";
  };

  const handlePointerUp = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (drag.kind === "node") {
      if (drag.moved) {
        suppressClickRef.current = true;
        const pruned = pruneRelationshipNodePositions(
          nodePositionsRef.current,
          layout.nodeById,
        );
        nodePositionsRef.current = pruned;
        setNodePositions(pruned);
        saveRelationshipNodePositions(projectId, pruned);
        setSelectedNodeId(drag.nodeId);
      }
    } else {
      suppressClickRef.current = true;
      if (!drag.moved) setSelectedNodeId(null);
    }
    if (canvasRef.current) canvasRef.current.style.cursor = "grab";
  };

  const handlePointerCancel = (
    event: React.PointerEvent<HTMLCanvasElement>,
  ) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (canvasRef.current) canvasRef.current.style.cursor = "default";
  };

  const handleCanvasClick = (event: React.MouseEvent<HTMLCanvasElement>) => {
    if (suppressClickRef.current) {
      suppressClickRef.current = false;
      return;
    }
    const point = canvasPoint(event, layout, viewport);
    if (!point) return;
    const hit = hitTestRelationshipGraph(layout, point.x, point.y);
    if (!hit) {
      setSelectedNodeId(null);
      return;
    }
    if (hit.kind === "node") {
      setSelectedNodeId((current) => (current === hit.id ? null : hit.id));
      setMobileView("characters");
      return;
    }
    const edge = layout.edges.find((item) => item.id === hit.id);
    if (edge) onOpen(edge.relationship);
  };

  const adjustViewportScale = (delta: number) => {
    setViewport((current) =>
      clampGraphViewport(
        {
          ...current,
          scale: clamp(
            Number((current.scale + delta).toFixed(2)),
            MIN_GRAPH_SCALE,
            MAX_GRAPH_SCALE,
          ),
        },
        layout,
      ),
    );
  };

  const resetNodeLayout = () => {
    nodePositionsRef.current = {};
    setNodePositions({});
    clearRelationshipNodePositions(projectId);
    setSelectedNodeId(null);
    setHoverTarget(null);
  };

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
            {layout.nodes.length} 位角色 · {layout.edges.length} 条关系
          </div>
        </div>
      </div>

      <div className="grid min-h-0 grid-cols-1 lg:flex-1 lg:grid-cols-[minmax(0,1fr)_288px]">
        <div
          ref={canvasHostRef}
          className={`${mobileView === "graph" ? "block" : "hidden"} relative h-[360px] min-h-0 min-w-0 bg-book-bg sm:h-[420px] lg:block lg:h-auto`}
        >
          <div className="absolute right-3 top-3 z-10 flex items-center gap-1 rounded-xl border border-book-border/65 bg-book-bg-paper/90 p-1 shadow-[var(--shadow-overlay)] backdrop-blur-xl">
            <button
              type="button"
              className="pyqt-btn-icon !h-10 !w-10 !rounded-lg sm:!h-8 sm:!w-8"
              onClick={() => adjustViewportScale(-0.16)}
              disabled={viewport.scale <= MIN_GRAPH_SCALE}
              aria-label="缩小关系图"
              title="缩小关系图"
            >
              <ZoomOut size={13} />
            </button>
            <button
              type="button"
              className="pyqt-btn-icon !h-10 !w-10 !rounded-lg sm:!h-8 sm:!w-8"
              onClick={() => adjustViewportScale(0.16)}
              disabled={viewport.scale >= MAX_GRAPH_SCALE}
              aria-label="放大关系图"
              title="放大关系图"
            >
              <ZoomIn size={13} />
            </button>
            {viewport.scale !== 1 ||
            viewport.offsetX !== 0 ||
            viewport.offsetY !== 0 ? (
              <button
                type="button"
                className="pyqt-btn-icon !h-10 !w-10 !rounded-lg sm:!h-8 sm:!w-8"
                onClick={() => setViewport(DEFAULT_GRAPH_VIEWPORT)}
                aria-label="还原关系图视图"
                title="还原关系图视图"
              >
                <RotateCcw size={13} />
              </button>
            ) : null}
            {fixedNodeCount > 0 ? (
              <Dropdown
                items={[
                  {
                    label: "恢复自动布局",
                    icon: <RefreshCcw size={13} />,
                    onClick: resetNodeLayout,
                  },
                ]}
              />
            ) : null}
          </div>
          <canvas
            ref={canvasRef}
            className="block h-full w-full"
            aria-label="当前角色关系网"
            title="拖动节点可固定位置，拖动画布可平移，点击关系查看详情"
            onClick={handleCanvasClick}
            onPointerDown={handlePointerDown}
            onPointerLeave={handlePointerLeave}
            onPointerMove={handlePointerMove}
            onPointerUp={handlePointerUp}
            onPointerCancel={handlePointerCancel}
          />
        </div>

        <RelationshipGraphPanel
          graph={layout}
          hoveredRelationship={hoveredRelationship}
          selectedNode={selectedNode}
          selectedRelationships={selectedRelationships}
          selectedNodeId={selectedNodeId}
          mobileVisible={mobileView === "characters"}
          onSelectNode={(nodeId) => setSelectedNodeId(nodeId)}
          onOpen={onOpen}
        />
      </div>
    </section>
  );
};

const RelationshipGraphPanel: React.FC<{
  graph: RelationshipGraphLayout;
  hoveredRelationship: CurrentRelationship | null;
  selectedNode: PositionedRelationshipNode | null;
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
    relationship.last_synced_chapter,
  );
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={`${relationship.character_a} / ${relationship.character_b}`}
      subtitle={`社会关系 · ${syncedChapterText(relationship.last_synced_chapter)}`}
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
            {relationship.last_synced_chapter > 0
              ? "该角色已经进入章节时间线。后续关系变化应在角色详情中按实际章节记录，不能直接重写初始关系。"
              : "该关系包含结构化资料，文本表单不会将其压平成普通描述。"}
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
  const ensureNode = (
    name: string,
    profileId: number | null,
  ): RelationshipNode => {
    const id = relationshipNodeId(name);
    const existing = nodeById.get(id);
    if (existing) {
      if (existing.profileId == null && profileId != null)
        existing.profileId = profileId;
      return existing;
    }
    const node: RelationshipNode = {
      id,
      name: name.trim(),
      profileId,
      degree: 0,
    };
    nodeById.set(id, node);
    relationshipsByNodeId.set(id, []);
    return node;
  };

  const edges = relationships.map((relationship) => {
    const from = ensureNode(relationship.character_a, relationship.profile_id);
    const to = ensureNode(relationship.character_b, null);
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
      parallelCount: 1,
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
      edge.parallelCount = group.length;
    });
  });

  return {
    nodes: [...nodeById.values()],
    edges,
    relationshipsByNodeId,
  };
}

function layoutRelationshipGraph(
  graph: RelationshipGraphData,
  size: CanvasSize,
  nodePositions: RelationshipNodePositions,
): RelationshipGraphLayout {
  const width = Math.max(MIN_CANVAS_WIDTH, size.width);
  const height = Math.max(MIN_CANVAS_HEIGHT, size.height);
  const centerX = width / 2;
  const centerY = height / 2;
  const maxDegree = Math.max(1, ...graph.nodes.map((node) => node.degree));
  const ordered = [...graph.nodes].sort(
    (a, b) => b.degree - a.degree || a.name.localeCompare(b.name),
  );
  const radius = Math.max(90, Math.min(width, height) * 0.34);
  const lockedNodeIds = new Set<string>();
  const nodes = ordered.map((node, index) => {
    const angle =
      (Math.PI * 2 * index) / Math.max(ordered.length, 1) - Math.PI / 2;
    const compactRadius = ordered.length <= 2 ? radius * 0.55 : radius;
    const nodeRadius = 18 + Math.round((node.degree / maxDegree) * 10);
    const saved = validRelationshipNodePosition(nodePositions[node.id]);
    if (saved) lockedNodeIds.add(node.id);
    return {
      ...node,
      x: saved
        ? clamp(saved.x * width, nodeRadius + 18, width - nodeRadius - 18)
        : centerX + Math.cos(angle) * compactRadius,
      y: saved
        ? clamp(saved.y * height, nodeRadius + 18, height - nodeRadius - 18)
        : centerY + Math.sin(angle) * compactRadius,
      radius: nodeRadius,
      colorIndex: index % RELATIONSHIP_NODE_COLOR_COUNT,
    };
  });
  if (nodes.length === 1 && !lockedNodeIds.has(nodes[0].id)) {
    nodes[0].x = centerX;
    nodes[0].y = centerY;
  } else if (nodes.length > 2) {
    relaxRelationshipLayout(nodes, graph.edges, width, height, lockedNodeIds);
  }
  const nodeById = new Map(nodes.map((node) => [node.id, node]));
  return {
    width,
    height,
    nodes,
    edges: graph.edges,
    nodeById,
    relationshipsByNodeId: graph.relationshipsByNodeId,
  };
}

function relaxRelationshipLayout(
  nodes: PositionedRelationshipNode[],
  edges: RelationshipEdge[],
  width: number,
  height: number,
  lockedNodeIds: Set<string>,
) {
  const nodeById = new Map(nodes.map((node) => [node.id, node]));
  const padding = 58;
  const desiredEdgeLength = Math.max(
    110,
    Math.min(190, Math.min(width, height) * 0.28),
  );
  const iterations = Math.min(180, 80 + nodes.length * 8);
  for (let iteration = 0; iteration < iterations; iteration += 1) {
    const dx = new Array(nodes.length).fill(0);
    const dy = new Array(nodes.length).fill(0);
    for (let i = 0; i < nodes.length; i += 1) {
      for (let j = i + 1; j < nodes.length; j += 1) {
        const a = nodes[i];
        const b = nodes[j];
        const deltaX = a.x - b.x || 0.01;
        const deltaY = a.y - b.y || 0.01;
        const distanceSq = Math.max(deltaX * deltaX + deltaY * deltaY, 100);
        const distance = Math.sqrt(distanceSq);
        const force = Math.min(8, 7000 / distanceSq);
        const pushX = (deltaX / distance) * force;
        const pushY = (deltaY / distance) * force;
        dx[i] += pushX;
        dy[i] += pushY;
        dx[j] -= pushX;
        dy[j] -= pushY;
      }
    }

    for (const edge of edges) {
      const from = nodeById.get(edge.fromId);
      const to = nodeById.get(edge.toId);
      if (!from || !to || from.id === to.id) continue;
      const fromIndex = nodes.indexOf(from);
      const toIndex = nodes.indexOf(to);
      const deltaX = to.x - from.x;
      const deltaY = to.y - from.y;
      const distance = Math.max(1, Math.hypot(deltaX, deltaY));
      const force = (distance - desiredEdgeLength) * 0.012;
      const pullX = (deltaX / distance) * force;
      const pullY = (deltaY / distance) * force;
      if (!lockedNodeIds.has(from.id)) {
        dx[fromIndex] += pullX;
        dy[fromIndex] += pullY;
      }
      if (!lockedNodeIds.has(to.id)) {
        dx[toIndex] -= pullX;
        dy[toIndex] -= pullY;
      }
    }

    const cooling = 0.92 - (iteration / iterations) * 0.55;
    for (let i = 0; i < nodes.length; i += 1) {
      const node = nodes[i];
      if (lockedNodeIds.has(node.id)) continue;
      const centerPullX = (width / 2 - node.x) * 0.006;
      const centerPullY = (height / 2 - node.y) * 0.006;
      node.x = clamp(
        node.x + (dx[i] + centerPullX) * cooling,
        padding,
        width - padding,
      );
      node.y = clamp(
        node.y + (dy[i] + centerPullY) * cooling,
        padding,
        height - padding,
      );
    }
  }
}

function drawRelationshipGraph(
  canvas: HTMLCanvasElement,
  layout: RelationshipGraphLayout,
  selectedNodeId: string | null,
  hoverTarget: RelationshipGraphHit | null,
  viewport: RelationshipGraphViewport,
) {
  const ratio = Math.max(1, Math.min(window.devicePixelRatio || 1, 2));
  canvas.width = Math.round(layout.width * ratio);
  canvas.height = Math.round(layout.height * ratio);
  canvas.style.width = `${layout.width}px`;
  canvas.style.height = `${layout.height}px`;

  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  const palette = canvasPalette();
  ctx.clearRect(0, 0, layout.width, layout.height);
  ctx.fillStyle = palette.background;
  ctx.fillRect(0, 0, layout.width, layout.height);
  ctx.save();
  applyGraphViewport(ctx, layout, viewport);
  drawCanvasGrid(ctx, layout, palette);

  for (const edge of layout.edges) {
    drawRelationshipEdge(
      ctx,
      layout,
      edge,
      selectedNodeId,
      hoverTarget,
      palette,
    );
  }
  for (const edge of layout.edges) {
    drawRelationshipEdgeLabel(
      ctx,
      layout,
      edge,
      selectedNodeId,
      hoverTarget,
      palette,
    );
  }
  for (const node of layout.nodes) {
    drawRelationshipNode(ctx, node, selectedNodeId, hoverTarget, palette);
  }
  drawRelationshipLabels(ctx, layout, selectedNodeId, hoverTarget, palette);
  ctx.restore();
}

function drawCanvasGrid(
  ctx: CanvasRenderingContext2D,
  layout: RelationshipGraphLayout,
  palette: ReturnType<typeof canvasPalette>,
) {
  ctx.save();
  ctx.strokeStyle = palette.grid;
  ctx.lineWidth = 1;
  for (let x = 32; x < layout.width; x += 32) {
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, layout.height);
    ctx.stroke();
  }
  for (let y = 32; y < layout.height; y += 32) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(layout.width, y);
    ctx.stroke();
  }
  ctx.restore();
}

function drawRelationshipEdge(
  ctx: CanvasRenderingContext2D,
  layout: RelationshipGraphLayout,
  edge: RelationshipEdge,
  selectedNodeId: string | null,
  hoverTarget: RelationshipGraphHit | null,
  palette: ReturnType<typeof canvasPalette>,
) {
  const from = layout.nodeById.get(edge.fromId);
  const to = layout.nodeById.get(edge.toId);
  if (!from || !to) return;
  const edgeHovered =
    hoverTarget?.kind === "edge" && hoverTarget.id === edge.id;
  const incidentSelected =
    selectedNodeId === from.id || selectedNodeId === to.id;
  const dimmed = selectedNodeId != null && !incidentSelected;
  const geometry = relationshipEdgeGeometry(from, to, edge);
  const emphasized = edgeHovered || incidentSelected;

  ctx.save();
  ctx.strokeStyle = emphasized
    ? palette.primary
    : dimmed
      ? palette.edgeDim
      : palette.edge;
  ctx.lineWidth = edgeHovered ? 2.6 : incidentSelected ? 2.1 : 1.3;
  ctx.globalAlpha = dimmed ? 0.28 : 1;
  ctx.beginPath();
  if (geometry.selfLoop) {
    ctx.arc(
      geometry.control.x,
      geometry.control.y,
      geometry.loopRadius ?? 24,
      geometry.loopStartAngle ?? 0,
      geometry.loopEndAngle ?? Math.PI * 2,
    );
  } else {
    ctx.moveTo(geometry.start.x, geometry.start.y);
    ctx.quadraticCurveTo(
      geometry.control.x,
      geometry.control.y,
      geometry.end.x,
      geometry.end.y,
    );
  }
  ctx.stroke();
  drawArrowHead(
    ctx,
    geometry.arrow.x,
    geometry.arrow.y,
    geometry.arrow.angle,
    emphasized ? 9 : 7,
  );
  ctx.restore();
}

function drawRelationshipEdgeLabel(
  ctx: CanvasRenderingContext2D,
  layout: RelationshipGraphLayout,
  edge: RelationshipEdge,
  selectedNodeId: string | null,
  hoverTarget: RelationshipGraphHit | null,
  palette: ReturnType<typeof canvasPalette>,
) {
  const from = layout.nodeById.get(edge.fromId);
  const to = layout.nodeById.get(edge.toId);
  if (!from || !to) return;
  const edgeHovered =
    hoverTarget?.kind === "edge" && hoverTarget.id === edge.id;
  const incidentSelected =
    selectedNodeId === from.id || selectedNodeId === to.id;
  const showLabel =
    edgeHovered ||
    incidentSelected ||
    (!selectedNodeId && layout.edges.length <= 6);
  if (!showLabel) return;

  const geometry = relationshipEdgeGeometry(from, to, edge);
  const anchor = geometry.selfLoop
    ? {
        x: geometry.control.x,
        y: geometry.control.y - (geometry.loopRadius ?? 24) - 6,
      }
    : quadraticPoint(geometry.start, geometry.control, geometry.end, 0.5);
  ctx.save();
  ctx.font =
    "600 10px system-ui, -apple-system, BlinkMacSystemFont, sans-serif";
  const summary = fitCanvasText(
    ctx,
    relationshipEdgeSummary(edge.relationship.description),
    edgeHovered ? 150 : 104,
  );
  const width = Math.min(
    edgeHovered ? 166 : 120,
    Math.max(40, ctx.measureText(summary).width + 16),
  );
  const height = 19;
  const x = clamp(anchor.x - width / 2, 8, layout.width - width - 8);
  const y = clamp(anchor.y - height / 2, 8, layout.height - height - 8);

  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.lineJoin = "round";
  ctx.strokeStyle = palette.textHalo;
  ctx.lineWidth = 4;
  ctx.strokeText(summary, x + width / 2, y + height / 2 + 0.5);
  ctx.fillStyle =
    edgeHovered || incidentSelected ? palette.primary : palette.muted;
  ctx.fillText(summary, x + width / 2, y + height / 2 + 0.5);
  ctx.restore();
}

function drawRelationshipNode(
  ctx: CanvasRenderingContext2D,
  node: PositionedRelationshipNode,
  selectedNodeId: string | null,
  hoverTarget: RelationshipGraphHit | null,
  palette: ReturnType<typeof canvasPalette>,
) {
  const selected = node.id === selectedNodeId;
  const hovered = hoverTarget?.kind === "node" && hoverTarget.id === node.id;
  const color = palette.nodeColors[node.colorIndex % palette.nodeColors.length];
  ctx.save();
  if (selected || hovered) {
    ctx.fillStyle = palette.primaryGlow;
    ctx.beginPath();
    ctx.arc(node.x, node.y, node.radius + 7, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.fillStyle = color.fill;
  ctx.strokeStyle = selected || hovered ? palette.primary : color.stroke;
  ctx.lineWidth = selected || hovered ? 2.4 : 1.4;
  ctx.beginPath();
  ctx.arc(node.x, node.y, node.radius, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();

  ctx.fillStyle = color.text;
  ctx.font =
    "700 13px system-ui, -apple-system, BlinkMacSystemFont, sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(initialOf(node.name), node.x, node.y + 0.5);
  ctx.restore();
}

function drawRelationshipLabels(
  ctx: CanvasRenderingContext2D,
  layout: RelationshipGraphLayout,
  selectedNodeId: string | null,
  hoverTarget: RelationshipGraphHit | null,
  palette: ReturnType<typeof canvasPalette>,
) {
  const placed: Array<{ x: number; y: number; width: number; height: number }> =
    [];
  const nodeBounds = layout.nodes.map((node) => ({
    id: node.id,
    rect: {
      x: node.x - node.radius - 4,
      y: node.y - node.radius - 4,
      width: (node.radius + 4) * 2,
      height: (node.radius + 4) * 2,
    },
  }));
  const hoveredNodeId = hoverTarget?.kind === "node" ? hoverTarget.id : null;
  const ordered = [...layout.nodes].sort((a, b) => {
    const priorityA = nodeLabelPriority(a, selectedNodeId, hoveredNodeId);
    const priorityB = nodeLabelPriority(b, selectedNodeId, hoveredNodeId);
    return (
      priorityB - priorityA ||
      b.degree - a.degree ||
      a.name.localeCompare(b.name)
    );
  });

  ctx.save();
  ctx.font =
    "600 11px system-ui, -apple-system, BlinkMacSystemFont, sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  for (const node of ordered) {
    const selected = node.id === selectedNodeId;
    const hovered = node.id === hoveredNodeId;
    const alwaysShow = selected || hovered || layout.nodes.length <= 8;
    if (
      !alwaysShow &&
      placed.length >= Math.min(12, Math.ceil(layout.nodes.length * 0.5))
    )
      continue;

    const label = fitCanvasText(ctx, node.name, selected || hovered ? 124 : 96);
    const labelWidth = Math.min(
      selected || hovered ? 136 : 108,
      Math.max(36, ctx.measureText(label).width + 14),
    );
    const labelHeight = 20;
    const belowY = node.y + node.radius + 8;
    const preferredY =
      belowY + labelHeight <= layout.height - 8
        ? belowY
        : node.y - node.radius - labelHeight - 8;
    const rect = {
      x: clamp(node.x - labelWidth / 2, 8, layout.width - labelWidth - 8),
      y: clamp(preferredY, 8, layout.height - labelHeight - 8),
      width: labelWidth,
      height: labelHeight,
    };
    const overlapsLabel = placed.some((item) => rectsOverlap(rect, item, 4));
    const overlapsOtherNode = nodeBounds.some(
      (item) => item.id !== node.id && rectsOverlap(rect, item.rect, 2),
    );
    if (!alwaysShow && (overlapsLabel || overlapsOtherNode)) continue;
    placed.push(rect);

    ctx.lineJoin = "round";
    ctx.strokeStyle = palette.textHalo;
    ctx.lineWidth = 4;
    ctx.strokeText(
      label,
      rect.x + rect.width / 2,
      rect.y + rect.height / 2 + 0.5,
    );
    ctx.fillStyle = selected || hovered ? palette.primary : palette.text;
    ctx.fillText(
      label,
      rect.x + rect.width / 2,
      rect.y + rect.height / 2 + 0.5,
    );
  }
  ctx.restore();
}

function nodeLabelPriority(
  node: PositionedRelationshipNode,
  selectedNodeId: string | null,
  hoveredNodeId: string | null,
): number {
  if (node.id === selectedNodeId) return 1000;
  if (node.id === hoveredNodeId) return 900;
  return node.degree;
}

function drawArrowHead(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  angle: number,
  size = 7,
) {
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(
    x - Math.cos(angle - Math.PI / 6) * size,
    y - Math.sin(angle - Math.PI / 6) * size,
  );
  ctx.lineTo(
    x - Math.cos(angle + Math.PI / 6) * size,
    y - Math.sin(angle + Math.PI / 6) * size,
  );
  ctx.closePath();
  ctx.fillStyle = ctx.strokeStyle;
  ctx.fill();
}

function hitTestRelationshipGraph(
  layout: RelationshipGraphLayout,
  x: number,
  y: number,
): RelationshipGraphHit | null {
  for (let index = layout.nodes.length - 1; index >= 0; index -= 1) {
    const node = layout.nodes[index];
    if (Math.hypot(x - node.x, y - node.y) <= node.radius + 9) {
      return { kind: "node", id: node.id };
    }
  }
  for (let index = layout.edges.length - 1; index >= 0; index -= 1) {
    const edge = layout.edges[index];
    const from = layout.nodeById.get(edge.fromId);
    const to = layout.nodeById.get(edge.toId);
    if (!from || !to) continue;
    if (distanceToRelationshipEdge(x, y, from, to, edge) <= 8) {
      return { kind: "edge", id: edge.id };
    }
  }
  return null;
}

function distanceToRelationshipEdge(
  x: number,
  y: number,
  from: PositionedRelationshipNode,
  to: PositionedRelationshipNode,
  edge: RelationshipEdge,
): number {
  const geometry = relationshipEdgeGeometry(from, to, edge);
  if (geometry.selfLoop) {
    return Math.abs(
      Math.hypot(x - geometry.control.x, y - geometry.control.y) -
        (geometry.loopRadius ?? 24),
    );
  }
  let min = Number.POSITIVE_INFINITY;
  let previous = geometry.start;
  for (let step = 1; step <= 24; step += 1) {
    const t = step / 24;
    const point = quadraticPoint(
      geometry.start,
      geometry.control,
      geometry.end,
      t,
    );
    min = Math.min(
      min,
      pointToSegmentDistance(x, y, previous.x, previous.y, point.x, point.y),
    );
    previous = point;
  }
  return min;
}

function relationshipEdgeGeometry(
  from: PositionedRelationshipNode,
  to: PositionedRelationshipNode,
  edge: RelationshipEdge,
): RelationshipEdgeGeometry {
  if (from.id === to.id) {
    const loopRadius = Math.max(24, from.radius + 6);
    const loopStartAngle = 0.2 * Math.PI;
    const loopEndAngle = 1.82 * Math.PI;
    const control = { x: from.x, y: from.y - from.radius - loopRadius * 0.72 };
    const arrow = {
      x: control.x + Math.cos(loopEndAngle) * loopRadius,
      y: control.y + Math.sin(loopEndAngle) * loopRadius,
      angle: loopEndAngle + Math.PI / 2,
    };
    return {
      start: { x: from.x + from.radius * 0.65, y: from.y - from.radius * 0.65 },
      end: { x: from.x - from.radius * 0.65, y: from.y - from.radius * 0.65 },
      control,
      arrow,
      selfLoop: true,
      loopRadius,
      loopStartAngle,
      loopEndAngle,
    };
  }
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const distance = Math.max(1, Math.hypot(dx, dy));
  const nx = dx / distance;
  const ny = dy / distance;
  const offset = (edge.parallelIndex - (edge.parallelCount - 1) / 2) * 18;
  const px = -ny;
  const py = nx;
  const start = {
    x: from.x + nx * (from.radius + 5) + px * offset,
    y: from.y + ny * (from.radius + 5) + py * offset,
  };
  const end = {
    x: to.x - nx * (to.radius + 7) + px * offset,
    y: to.y - ny * (to.radius + 7) + py * offset,
  };
  const control = {
    x: (from.x + to.x) / 2 + px * offset * 1.8,
    y: (from.y + to.y) / 2 + py * offset * 1.8,
  };
  const arrow = quadraticPoint(start, control, end, 0.9);
  const beforeArrow = quadraticPoint(start, control, end, 0.84);
  return {
    start,
    end,
    control,
    arrow: {
      x: arrow.x,
      y: arrow.y,
      angle: Math.atan2(arrow.y - beforeArrow.y, arrow.x - beforeArrow.x),
    },
    selfLoop: false,
  };
}

function canvasPoint(
  event:
    React.MouseEvent<HTMLCanvasElement> | React.PointerEvent<HTMLCanvasElement>,
  layout: RelationshipGraphLayout,
  viewport: RelationshipGraphViewport,
): { x: number; y: number } | null {
  const rect = event.currentTarget.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return null;
  const screenX = ((event.clientX - rect.left) / rect.width) * layout.width;
  const screenY = ((event.clientY - rect.top) / rect.height) * layout.height;
  return graphPointFromScreen(screenX, screenY, layout, viewport);
}

function relationshipGraphCursor(hit: RelationshipGraphHit | null): string {
  if (hit?.kind === "node") return "grab";
  if (hit?.kind === "edge") return "pointer";
  return "grab";
}

function clampRelationshipNodePosition(
  x: number,
  y: number,
  layout: RelationshipGraphLayout,
  nodeId: string,
): { x: number; y: number } {
  const node = layout.nodeById.get(nodeId);
  const padding = Math.min(
    (node?.radius ?? 20) + 18,
    Math.min(layout.width, layout.height) / 2,
  );
  return {
    x: clamp(x, padding, layout.width - padding),
    y: clamp(y, padding, layout.height - padding),
  };
}

function applyGraphViewport(
  ctx: CanvasRenderingContext2D,
  layout: RelationshipGraphLayout,
  viewport: RelationshipGraphViewport,
) {
  ctx.translate(
    layout.width / 2 + viewport.offsetX,
    layout.height / 2 + viewport.offsetY,
  );
  ctx.scale(viewport.scale, viewport.scale);
  ctx.translate(-layout.width / 2, -layout.height / 2);
}

function graphPointFromScreen(
  x: number,
  y: number,
  layout: RelationshipGraphLayout,
  viewport: RelationshipGraphViewport,
): { x: number; y: number } {
  return {
    x:
      (x - layout.width / 2 - viewport.offsetX) / viewport.scale +
      layout.width / 2,
    y:
      (y - layout.height / 2 - viewport.offsetY) / viewport.scale +
      layout.height / 2,
  };
}

function relationshipLayoutStorageKey(projectId: number): string {
  return `${RELATIONSHIP_GRAPH_LAYOUT_STORAGE_PREFIX}:${projectId}`;
}

function loadRelationshipNodePositions(
  projectId: number,
): RelationshipNodePositions {
  try {
    const raw = readPortableItem(relationshipLayoutStorageKey(projectId));
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    const positions = asRelationshipPositionRecord(parsed);
    return positions ? sanitizeRelationshipNodePositions(positions) : {};
  } catch {
    return {};
  }
}

function saveRelationshipNodePositions(
  projectId: number,
  positions: RelationshipNodePositions,
) {
  const sanitized = sanitizeRelationshipNodePositions(positions);
  if (Object.keys(sanitized).length === 0) {
    clearRelationshipNodePositions(projectId);
    return;
  }
  try {
    writePortableItem(
      relationshipLayoutStorageKey(projectId),
      JSON.stringify({
        schema_version: 1,
        updated_at: new Date().toISOString(),
        positions: sanitized,
      }),
    );
  } catch (error) {
    console.warn("保存关系图节点布局失败", error);
  }
}

function clearRelationshipNodePositions(projectId: number) {
  try {
    removePortableItem(relationshipLayoutStorageKey(projectId));
  } catch (error) {
    console.warn("清理关系图节点布局失败", error);
  }
}

function asRelationshipPositionRecord(value: unknown): unknown {
  if (!value || typeof value !== "object") return null;
  const record = value as Record<string, unknown>;
  return record.positions && typeof record.positions === "object"
    ? record.positions
    : value;
}

function sanitizeRelationshipNodePositions(
  value: unknown,
): RelationshipNodePositions {
  if (!value || typeof value !== "object") return {};
  const output: RelationshipNodePositions = {};
  Object.entries(value as Record<string, unknown>).forEach(
    ([nodeId, rawPosition]) => {
      const position = validRelationshipNodePosition(rawPosition);
      const normalizedNodeId = nodeId.trim();
      if (!normalizedNodeId || !position) return;
      output[normalizedNodeId] = position;
    },
  );
  return output;
}

function pruneRelationshipNodePositions(
  positions: RelationshipNodePositions,
  nodeById: Map<string, PositionedRelationshipNode>,
): RelationshipNodePositions {
  const output: RelationshipNodePositions = {};
  Object.entries(positions).forEach(([nodeId, position]) => {
    const valid = validRelationshipNodePosition(position);
    if (!nodeById.has(nodeId) || !valid) return;
    output[nodeId] = valid;
  });
  return output;
}

function validRelationshipNodePosition(
  value: unknown,
): RelationshipNodePosition | null {
  if (!value || typeof value !== "object") return null;
  const record = value as Record<string, unknown>;
  const x = Number(record.x);
  const y = Number(record.y);
  if (!Number.isFinite(x) || !Number.isFinite(y)) return null;
  return {
    x: clamp(x, 0, 1),
    y: clamp(y, 0, 1),
  };
}

function sameRelationshipNodePositions(
  left: RelationshipNodePositions,
  right: RelationshipNodePositions,
): boolean {
  const leftKeys = Object.keys(left);
  const rightKeys = Object.keys(right);
  if (leftKeys.length !== rightKeys.length) return false;
  return leftKeys.every(
    (key) => left[key]?.x === right[key]?.x && left[key]?.y === right[key]?.y,
  );
}

function clampGraphViewport(
  viewport: RelationshipGraphViewport,
  layout: RelationshipGraphLayout,
): RelationshipGraphViewport {
  const scale = clamp(viewport.scale, MIN_GRAPH_SCALE, MAX_GRAPH_SCALE);
  const maxOffsetX = (layout.width * Math.max(scale - 1, 0)) / 2 + 120;
  const maxOffsetY = (layout.height * Math.max(scale - 1, 0)) / 2 + 120;
  return {
    scale,
    offsetX: clamp(viewport.offsetX, -maxOffsetX, maxOffsetX),
    offsetY: clamp(viewport.offsetY, -maxOffsetY, maxOffsetY),
  };
}

function canvasPalette() {
  const root = getComputedStyle(document.documentElement);
  const primary = readCssRgbVariable(root, "--color-primary", [139, 69, 19]);
  const accent = readCssRgbVariable(root, "--color-accent", [160, 82, 45]);
  const info = readCssRgbVariable(root, "--status-info", [74, 141, 179]);
  const success = readCssRgbVariable(root, "--status-success", [74, 159, 110]);
  const warning = readCssRgbVariable(root, "--status-warning", [212, 146, 58]);
  const bg = readCssRgbVariable(root, "--color-bg-primary", [249, 245, 240]);
  const border = readCssRgbVariable(root, "--color-border", [215, 204, 200]);
  const text = readCssRgbVariable(root, "--color-text-primary", [44, 24, 16]);
  const muted = readCssRgbVariable(root, "--color-text-secondary", [93, 64, 55]);
  const colors = [primary, info, success, warning, accent];
  return {
    background: rgba(bg, 1),
    grid: rgba(border, 0.22),
    border: rgba(border, 0.9),
    primary: rgba(primary, 1),
    primaryGlow: rgba(primary, 0.16),
    edge: rgba(muted, 0.42),
    edgeDim: rgba(muted, 0.22),
    text: rgba(text, 1),
    textHalo: rgba(bg, 0.92),
    muted: rgba(muted, 1),
    nodeColors: colors.map((rgb) => ({
      fill: rgba(rgb, 0.16),
      stroke: rgba(rgb, 0.72),
      text: rgba(rgb, 1),
    })),
  };
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

function sameHit(
  left: RelationshipGraphHit | null,
  right: RelationshipGraphHit | null,
): boolean {
  return left?.kind === right?.kind && left?.id === right?.id;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function quadraticPoint(
  start: { x: number; y: number },
  control: { x: number; y: number },
  end: { x: number; y: number },
  t: number,
): { x: number; y: number } {
  const inverse = 1 - t;
  return {
    x:
      inverse * inverse * start.x + 2 * inverse * t * control.x + t * t * end.x,
    y:
      inverse * inverse * start.y + 2 * inverse * t * control.y + t * t * end.y,
  };
}

function pointToSegmentDistance(
  px: number,
  py: number,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): number {
  const dx = x2 - x1;
  const dy = y2 - y1;
  if (dx === 0 && dy === 0) return Math.hypot(px - x1, py - y1);
  const t = clamp(
    ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy),
    0,
    1,
  );
  return Math.hypot(px - (x1 + t * dx), py - (y1 + t * dy));
}

function rectsOverlap(
  left: { x: number; y: number; width: number; height: number },
  right: { x: number; y: number; width: number; height: number },
  gap = 0,
): boolean {
  return !(
    left.x + left.width + gap < right.x ||
    right.x + right.width + gap < left.x ||
    left.y + left.height + gap < right.y ||
    right.y + right.height + gap < left.y
  );
}

function fitCanvasText(
  ctx: CanvasRenderingContext2D,
  text: string,
  maxWidth: number,
): string {
  if (ctx.measureText(text).width <= maxWidth) return text;
  const ellipsis = "…";
  let result = text;
  while (
    result.length > 1 &&
    ctx.measureText(`${result}${ellipsis}`).width > maxWidth
  ) {
    result = result.slice(0, -1);
  }
  return `${result}${ellipsis}`;
}
