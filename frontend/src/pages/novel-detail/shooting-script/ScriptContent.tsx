import React from "react";
import { ChevronRight } from "lucide-react";

import type { ChapterShootingScriptView } from "../../../api/shootingScript";
import {
  shootingCameraAngleLabel,
  shootingCameraMovementLabel,
  shootingCoverageLabel,
  shootingShotSizeLabel,
} from "../../../utils/shootingScriptPresentation";

export const ScriptContent: React.FC<{
  view: ChapterShootingScriptView;
}> = ({ view }) => (
  <div className="space-y-3">
    <section className="a4n-flat-surface overflow-hidden">
      <div className="border-b border-book-border/45 px-3 py-2.5 sm:px-4">
        <h3 className="text-sm font-semibold text-book-text-main">导演意图</h3>
      </div>
      <div className="divide-y divide-book-border/45 px-3 sm:px-4">
        <Info
          label="核心戏剧问题"
          value={view.script.directing_statement.central_dramatic_question}
        />
        <Info
          label="主角行动主线"
          value={view.script.directing_statement.protagonist_throughline}
        />
        <Info
          label="观众体验"
          value={view.script.directing_statement.audience_experience}
        />
        <Info
          label="视觉策略"
          value={view.script.directing_statement.visual_strategy}
        />
      </div>
      <details className="group border-t border-book-border/45 px-3 py-2 sm:px-4">
        <summary className="flex min-h-9 w-fit cursor-pointer list-none items-center gap-1.5 rounded-lg px-2 text-xs text-book-text-muted transition-colors hover:bg-book-bg-tertiary/60 hover:text-book-text-main [&::-webkit-details-marker]:hidden">
          <ChevronRight
            size={13}
            className="transition-transform group-open:rotate-90"
          />
          更多创作策略
        </summary>
        <div className="mt-1 divide-y divide-book-border/45 border-t border-book-border/45">
          <Info
            label="视点策略"
            value={view.script.directing_statement.viewpoint_strategy}
          />
          <Info
            label="空间策略"
            value={view.script.directing_statement.spatial_strategy}
          />
          <Info
            label="声音策略"
            value={view.script.directing_statement.sound_strategy}
          />
          <Info
            label="节奏策略"
            value={view.script.directing_statement.rhythm_strategy}
          />
        </div>
      </details>
    </section>

    <div className="space-y-3">
      {view.script.scenes.map((scene) => (
        <article
          key={scene.scene_index}
          className="a4n-flat-surface overflow-hidden"
        >
          <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1 border-b border-book-border/45 px-3 py-2.5 sm:px-4">
            <h3 className="text-sm font-semibold text-book-text-main">
              场景 {scene.scene_index} · {scene.title}
            </h3>
            <span className="text-[11px] text-book-text-muted">
              {scene.shots.length} 个镜头
            </span>
            <div className="w-full text-[11px] text-book-text-muted">
              {scene.slugline}
            </div>
          </div>
          <div className="grid gap-2 border-b border-book-border/40 px-3 py-3 text-xs sm:grid-cols-2 sm:px-4">
            <Info label="场景目标" value={scene.scene_objective} compact />
            <Info label="主要阻碍" value={scene.scene_obstacle} compact />
          </div>

          {scene.shots.length > 0 ? (
            <div className="divide-y divide-book-border/45 px-3 sm:px-4">
              {scene.shots.map((shot) => (
                <details key={shot.shot_id} className="group">
                  <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-3 py-3 text-left [&::-webkit-details-marker]:hidden">
                    <div className="min-w-0">
                      <div className="truncate text-xs font-semibold text-book-text-main">
                        镜头 {shot.shot_index} · {shot.title}
                      </div>
                      <div className="mt-1 flex flex-wrap gap-x-2 gap-y-0.5 text-[10px] text-book-text-muted">
                        <span>{shootingShotSizeLabel(shot.shot_size)}</span>
                        <span>{shootingCameraAngleLabel(shot.camera_angle)}</span>
                        <span>
                          {shootingCameraMovementLabel(shot.camera_movement)}
                        </span>
                        <span>{shootingCoverageLabel(shot.coverage)}</span>
                        <span>
                          {shot.estimated_natural_seconds.toFixed(1)} 秒
                        </span>
                      </div>
                    </div>
                    <ChevronRight
                      size={14}
                      className="text-book-text-muted transition-transform group-open:rotate-90"
                    />
                  </summary>
                  <div className="pb-3 pl-2 sm:pl-3">
                    <p className="border-l-2 border-book-primary/25 pl-3 text-xs leading-relaxed text-book-text-sub">
                      {shot.composition}
                    </p>
                    {shot.performance_beats.length > 0 ? (
                      <div className="mt-3 divide-y divide-book-border/35 border-y border-book-border/35">
                        {shot.performance_beats.map((beat) => (
                          <div
                            key={beat.beat_id}
                            className="grid gap-1 py-2 text-xs sm:grid-cols-[72px_minmax(0,1fr)]"
                          >
                            <span className="text-book-text-muted">
                              节拍 {beat.beat_index}
                            </span>
                            <div className="min-w-0 text-book-text-main">
                              {beat.actor ? `${beat.actor}：` : ""}
                              {beat.visible_action}
                              {beat.reaction ? (
                                <div className="mt-0.5 text-[11px] text-book-text-muted">
                                  反应 · {beat.reaction}
                                </div>
                              ) : null}
                            </div>
                          </div>
                        ))}
                      </div>
                    ) : null}
                  </div>
                </details>
              ))}
            </div>
          ) : (
            <div className="px-3 py-4 text-xs text-book-text-muted sm:px-4">
              该场景尚未拆分镜头。
            </div>
          )}
        </article>
      ))}
    </div>
  </div>
);

const Info: React.FC<{
  label: string;
  value: string;
  compact?: boolean;
}> = ({ label, value, compact = false }) => (
  <div
    className={
      compact
        ? "grid gap-1"
        : "grid gap-1 py-2.5 sm:grid-cols-[112px_minmax(0,1fr)] sm:gap-3"
    }
  >
    <div className="text-[11px] text-book-text-muted">{label}</div>
    <div className="text-xs leading-relaxed text-book-text-main">{value}</div>
  </div>
);
