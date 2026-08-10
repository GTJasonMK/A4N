type AdminServiceMode = "embedded" | "sidecar" | "remote" | "disabled" | string;
type AdminServicePidStatus =
  | "observed"
  | "missing"
  | "stale"
  | "invalid"
  | "unreadable"
  | "unverifiable"
  | string;

export interface AdminServiceHealth {
  name: string;
  mode: AdminServiceMode;
  service_mode?: AdminServiceMode;
  url: string | null;
  port: number | null;
  health_path: string | null;
  binary: string | null;
  pid: number | null;
  pid_status?: AdminServicePidStatus | null;
  pid_source?: string | null;
  healthy: boolean;
  status: string | null;
  service_version: string | null;
  protocol_version: string | null;
  capability_version: string | null;
  error_code: string | null;
  error_message: string | null;
}

export interface AdminHealthCheck {
  runtime_profile?: string;
  services?: AdminServiceHealth[];
  [key: string]: unknown;
}
