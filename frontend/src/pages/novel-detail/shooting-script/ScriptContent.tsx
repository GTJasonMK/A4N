import React from "react";
import { ChevronRight } from "lucide-react";

import type { ChapterShootingScriptV1 } from "../../../api/shootingScript";
import {
  shootingCameraAngleLabel,
  shootingCameraMovementLabel,
  shootingCoverageLabel,
  shootingShotSizeLabel,
} from "../../../utils/shootingScriptPresentation";
import { formatNumber, useI18n } from "../../../i18n";

export const ScriptContent: React.FC<{
  script: ChapterShootingScriptV1;
}> = ({ script }) => {
  const { locale, t } = useI18n();
  return (
  <div className="space-y-3">
    <section className="a4n-flat-surface overflow-hidden">
      <div className="border-b border-book-border/45 px-3 py-2.5 sm:px-4">
        <h3 className="text-sm font-semibold text-book-text-main">{t("shootingScript.editor.directingIntent")}</h3>
      </div>
      <div className="divide-y divide-book-border/45 px-3 sm:px-4">
        <Info
          label={t("shootingScript.editor.centralDramaticQuestion")}
          value={script.directing_statement.central_dramatic_question}
        />
        <Info
          label={t("shootingScript.editor.protagonistThroughline")}
          value={script.directing_statement.protagonist_throughline}
        />
        <Info
          label={t("shootingScript.editor.audienceExperience")}
          value={script.directing_statement.audience_experience}
        />
        <Info
          label={t("shootingScript.editor.visualStrategy")}
          value={script.directing_statement.visual_strategy}
        />
      </div>
      <details className="group border-t border-book-border/45 px-3 py-2 sm:px-4">
        <summary className="flex min-h-9 w-fit cursor-pointer list-none items-center gap-1.5 rounded-lg px-2 text-xs text-book-text-muted transition-colors hover:bg-book-bg-tertiary/60 hover:text-book-text-main [&::-webkit-details-marker]:hidden">
          <ChevronRight
            size={13}
            className="transition-transform group-open:rotate-90"
          />
          {t("shootingScript.moreStrategies")}
        </summary>
        <div className="mt-1 divide-y divide-book-border/45 border-t border-book-border/45">
          <Info
            label={t("shootingScript.editor.viewpointStrategy")}
            value={script.directing_statement.viewpoint_strategy}
          />
          <Info
            label={t("shootingScript.editor.spatialStrategy")}
            value={script.directing_statement.spatial_strategy}
          />
          <Info
            label={t("shootingScript.editor.soundStrategy")}
            value={script.directing_statement.sound_strategy}
          />
          <Info
            label={t("shootingScript.editor.rhythmStrategy")}
            value={script.directing_statement.rhythm_strategy}
          />
        </div>
      </details>
    </section>

    <div className="space-y-3">
      {script.scenes.map((scene) => (
        <article
          key={scene.scene_index}
          className="a4n-flat-surface overflow-hidden"
        >
          <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1 border-b border-book-border/45 px-3 py-2.5 sm:px-4">
            <h3 className="text-sm font-semibold text-book-text-main">
              {t("shootingScript.editor.scene", { index: scene.scene_index })} · {scene.title}
            </h3>
            <span className="text-small text-book-text-muted">
              {t("shootingScript.editor.shotsCount", { count: scene.shots.length })}
            </span>
            <div className="w-full text-small text-book-text-muted">
              {scene.slugline}
            </div>
          </div>
          <div className="grid gap-2 border-b border-book-border/40 px-3 py-3 text-xs sm:grid-cols-2 sm:px-4">
            <Info label={t("shootingScript.editor.sceneObjective")} value={scene.scene_objective} compact />
            <Info label={t("shootingScript.editor.sceneObstacle")} value={scene.scene_obstacle} compact />
          </div>

          {scene.shots.length > 0 ? (
            <div className="divide-y divide-book-border/45 px-3 sm:px-4">
              {scene.shots.map((shot) => (
                <details key={shot.shot_id} className="group">
                  <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-3 py-3 text-left [&::-webkit-details-marker]:hidden">
                    <div className="min-w-0">
                      <div className="truncate text-xs font-semibold text-book-text-main">
                        {t("shootingScript.editor.shot")} {shot.shot_index} · {shot.title}
                      </div>
                      <div className="mt-1 flex flex-wrap gap-x-2 gap-y-0.5 text-caption text-book-text-muted">
                        <span>{shootingShotSizeLabel(shot.shot_size, t)}</span>
                        <span>{shootingCameraAngleLabel(shot.camera_angle, t)}</span>
                        <span>
                          {shootingCameraMovementLabel(shot.camera_movement, t)}
                        </span>
                        <span>{shootingCoverageLabel(shot.coverage, t)}</span>
                        <span>
                          {formatNumber(shot.estimated_natural_seconds, locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} {t("units.seconds")}
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
                              {t("shootingScript.editor.beat", { index: beat.beat_index })}
                            </span>
                            <div className="min-w-0 text-book-text-main">
                              {beat.actor ? `${beat.actor}：` : ""}
                              {beat.visible_action}
                              {beat.reaction ? (
                                <div className="mt-0.5 text-small text-book-text-muted">
                                  {t("shootingScript.editor.reaction")} · {beat.reaction}
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
              {t("shootingScript.noShots")}
            </div>
          )}
        </article>
      ))}
    </div>
  </div>
  );
};

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
    <div className="text-small text-book-text-muted">{label}</div>
    <div className="text-xs leading-relaxed text-book-text-main">{value}</div>
  </div>
);
