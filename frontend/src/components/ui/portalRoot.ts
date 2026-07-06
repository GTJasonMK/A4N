const PORTAL_ROOT_ID = "a4n-portal-root";

export const getPortalRoot = (): HTMLElement => {
  if (typeof document === "undefined") {
    throw new Error("Portal root is unavailable outside the browser document");
  }
  const root = document.getElementById(PORTAL_ROOT_ID);
  if (!root) {
    throw new Error(`Missing portal root #${PORTAL_ROOT_ID}`);
  }
  return root;
};
