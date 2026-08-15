import { describe, expect, it } from "vitest";
import {
  formalReverseNotice,
  importAnalysisDialogFromProgress,
} from "./importTabUtils";

describe("import tab presentation", () => {
  it("maps strict import analysis progress to the completed dialog state", () => {
    expect(
      importAnalysisDialogFromProgress({
        cancelled_at: null,
        completed_at: "now",
        current_stage: "generating_analysis_data",
        error: null,
        failed_at: null,
        message: "导入项目分析已完成",
        overall_progress: 100,
        stages: {
          generating_analysis_data: {
            completed: 3,
            name: "生成分析数据",
            status: "completed",
            total: 3,
          },
        },
        started_at: "now",
        status: "completed",
      }),
    ).toEqual({
      open: true,
      status: "success",
      current: 3,
      total: 3,
      message: "导入项目分析已完成",
      error: null,
    });
  });

  it("reports committed material sync warnings as information", () => {
    expect(formalReverseNotice("蓝图已归纳并写入正式资料", ["资料检索索引更新失败"])).toEqual({
      message: "蓝图已归纳并写入正式资料；资料检索索引更新失败",
      tone: "info",
    });
  });
});
