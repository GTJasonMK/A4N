import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Loader2, Upload } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import type { CharacterProfile } from "../../../api/character";
import { uploadPortrait } from "../../../api/portrait";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput, BookTextarea } from "../../../components/ui/BookInput";
import { Modal } from "../../../components/ui/Modal";
import {
  isSupportedImageMime,
  MAX_UPLOAD_IMAGE_BYTES,
  readFileAsBase64,
  supportedImageAccept,
} from "../../../utils/imageUpload";

interface UploadPortraitDialogProps {
  projectId: number;
  profiles: CharacterProfile[];
  onClose: () => void;
  onUploaded: () => void;
}

const PORTRAIT_TYPE_OPTIONS = [
  { value: "full_body", label: "全身三视图拆解" },
  { value: "half_body", label: "半身三视图拆解" },
  { value: "face", label: "面部/发型拆解" },
  { value: "chibi", label: "Q版三视图拆解" },
] as const;

export const UploadPortraitDialog: React.FC<UploadPortraitDialogProps> = ({
  projectId,
  profiles,
  onClose,
  onUploaded,
}) => {
  const addToast = useToast((state) => state.addToast);
  const [profileId, setProfileId] = useState(profiles[0]?.id ?? 0);
  const [portraitType, setPortraitType] = useState("full_body");
  const [prompt, setPrompt] = useState("");
  const [expression, setExpression] = useState("");
  const [pose, setPose] = useState("");
  const [setPrimary, setSetPrimary] = useState(true);
  const [file, setFile] = useState<File | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const activeProjectIdRef = useRef(projectId);
  const mountedRef = useRef(true);
  const submitBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      submitBusyRef.current = false;
    };
  }, []);

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === requestProjectId;
  }, []);

  const selectedProfile = useMemo(
    () => profiles.find((profile) => profile.id === profileId) ?? null,
    [profileId, profiles],
  );

  useEffect(() => {
    setProfileId((current) => {
      if (profiles.some((profile) => profile.id === current)) return current;
      return profiles[0]?.id ?? 0;
    });
  }, [profiles]);

  useEffect(() => {
    return () => {
      if (previewUrl) URL.revokeObjectURL(previewUrl);
    };
  }, [previewUrl]);

  const handleFileChange = async (nextFile: File | null) => {
    if (!nextFile) return;
    if (!isSupportedImageMime(nextFile.type)) {
      addToast("只支持 PNG/JPEG/WebP 图片", "error");
      return;
    }
    if (nextFile.size > MAX_UPLOAD_IMAGE_BYTES) {
      addToast(`上传图片不能超过 ${MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024}MB`, "error");
      return;
    }
    setFile(nextFile);
    setPreviewUrl((current) => {
      if (current) URL.revokeObjectURL(current);
      return URL.createObjectURL(nextFile);
    });
    if (!prompt.trim()) {
      const name = selectedProfile?.name ?? "角色";
      setPrompt(`用户上传角色立绘：${name}，来源文件：${nextFile.name}`);
    }
  };

  const handleSubmit = async () => {
    if (submitBusyRef.current) return;
    if (!selectedProfile) {
      addToast("请选择角色档案", "error");
      return;
    }
    if (!file) {
      addToast("请选择要上传的立绘图片", "error");
      return;
    }
    if (!prompt.trim()) {
      addToast("请填写立绘描述", "error");
      return;
    }
    const requestProjectId = projectId;
    const requestProfile = selectedProfile;
    const requestFile = file;
    const requestPayload = {
      portraitType,
      prompt: prompt.trim(),
      expression: expression.trim() || null,
      pose: pose.trim() || null,
      setPrimary,
    };
    submitBusyRef.current = true;
    setBusy(true);
    try {
      const dataBase64 = await readFileAsBase64(requestFile);
      if (!isCurrentProject(requestProjectId)) return;
      await uploadPortrait({
        project_id: requestProjectId,
        profile_id: requestProfile.id,
        file_name: requestFile.name,
        mime_type: requestFile.type,
        data_base64: dataBase64,
        portrait_type: requestPayload.portraitType,
        prompt: requestPayload.prompt,
        expression: requestPayload.expression,
        pose: requestPayload.pose,
        set_primary: requestPayload.setPrimary,
      });
      if (!isCurrentProject(requestProjectId)) return;
      addToast("已上传角色立绘", "success");
      onUploaded();
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      addToast(extractErrorMessage(err, "上传角色立绘失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) {
        submitBusyRef.current = false;
        setBusy(false);
      }
    }
  };

  return (
    <Modal isOpen onClose={onClose} title="上传角色立绘" maxWidthClassName="max-w-2xl">
      <div className="space-y-3 text-sm">
        {profiles.length === 0 ? (
          <div className="rounded border border-book-border/50 bg-book-bg-paper/45 px-3 py-2 text-xs text-book-text-muted">
            当前项目还没有角色档案，需先创建或同步角色档案后再上传立绘。
          </div>
        ) : null}

        <div>
          <label className="mb-1 block text-xs text-book-text-muted">角色档案</label>
          <select
            value={profileId}
            onChange={(event) => setProfileId(Number(event.target.value))}
            className="pyqt-input w-full"
            disabled={busy || profiles.length === 0}
          >
            {profiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="mb-1 block text-xs text-book-text-muted">立绘类型</label>
          <select
            value={portraitType}
            onChange={(event) => setPortraitType(event.target.value)}
            className="pyqt-input w-full"
            disabled={busy}
          >
            {PORTRAIT_TYPE_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        <label
          className={[
            "flex min-h-[160px] cursor-pointer flex-col items-center justify-center gap-2 rounded border border-dashed border-book-border/70 bg-book-bg-paper/45 p-3 text-center text-xs text-book-text-muted transition-colors hover:border-book-primary/70",
            busy ? "pointer-events-none opacity-60" : "",
          ].join(" ")}
        >
          {previewUrl ? (
            <img
              src={previewUrl}
              alt=""
              className="max-h-56 max-w-full rounded border border-book-border/50 object-contain"
            />
          ) : (
            <>
              <Upload size={18} />
              <span>选择 PNG/JPEG/WebP 本地图片作为最终角色立绘</span>
            </>
          )}
          {file ? <span className="font-mono text-[10px]">{file.name}</span> : null}
          <input
            type="file"
            accept={supportedImageAccept()}
            className="hidden"
            disabled={busy}
            onChange={(event) => {
              const nextFile = event.currentTarget.files?.[0] ?? null;
              event.currentTarget.value = "";
              void handleFileChange(nextFile);
            }}
          />
        </label>

        <BookTextarea
          label="立绘描述"
          value={prompt}
          rows={3}
          disabled={busy}
          onChange={(event) => setPrompt(event.target.value)}
          placeholder="描述这张立绘的角色、服装、姿态、风格和用途"
        />

        <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
          <BookInput
            label="表情（可空）"
            value={expression}
            disabled={busy}
            onChange={(event) => setExpression(event.target.value)}
            placeholder="如：neutral / 微笑 / 严肃"
          />
          <BookInput
            label="姿态（可空）"
            value={pose}
            disabled={busy}
            onChange={(event) => setPose(event.target.value)}
            placeholder="如：standing / 半身正面"
          />
        </div>

        <label className="inline-flex items-center gap-2 text-xs text-book-text-muted">
          <input
            type="checkbox"
            checked={setPrimary}
            disabled={busy}
            onChange={(event) => setSetPrimary(event.target.checked)}
          />
          设为该角色主图
        </label>

        <div className="flex justify-end gap-2 pt-2">
          <BookButton variant="secondary" onClick={onClose} disabled={busy}>
            取消
          </BookButton>
          <BookButton
            variant="primary"
            onClick={handleSubmit}
            disabled={busy || profiles.length === 0}
          >
            {busy ? (
              <span className="inline-flex items-center gap-1.5">
                <Loader2 size={12} className="animate-spin" />
                上传中…
              </span>
            ) : (
              "上传立绘"
            )}
          </BookButton>
        </div>
      </div>
    </Modal>
  );
};
