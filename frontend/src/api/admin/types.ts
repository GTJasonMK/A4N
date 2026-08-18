type AdminRuntimeProfile = "full" | "minimal" | "sidecar";
type AdminServiceMode = "embedded" | "sidecar" | "remote" | "disabled";

export interface AdminServiceHealth {
  name: string;
  mode: AdminServiceMode;
  url: string | null;
  binary: string | null;
  pid: number | null;
  healthy: boolean;
  service_version: string | null;
  protocol_version: number | null;
  capability_version: string | null;
  error_message: string | null;
}

export interface AdminHealthCheck {
  runtime_profile: AdminRuntimeProfile;
  services: AdminServiceHealth[];
}
