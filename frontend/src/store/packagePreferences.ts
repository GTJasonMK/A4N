import { useCallback, useEffect } from "react";
import { create } from "zustand";
import { extractErrorMessage } from "../api";
import { getPackagePreferences, updatePackagePreferences } from "../api/package";
import type { PackageId, PackagePreferencesSummary } from "../api/generated/page";
import { getActiveLocale, translateMessage } from "../i18n";
import { useAuthStore } from "./auth";

interface PackagePreferencesState {
  userId: number | null;
  preferences: PackagePreferencesSummary | null;
  error: string | null;
}

export const usePackagePreferencesStore = create<PackagePreferencesState>(() => ({
  userId: null,
  preferences: null,
  error: null,
}));

let requestVersion = 0;
let pending: { userId: number; promise: Promise<void> } | null = null;

export function refreshPackagePreferences(userId: number): Promise<void> {
  if (pending?.userId === userId) return pending.promise;
  const version = ++requestVersion;
  const previous = usePackagePreferencesStore.getState();
  usePackagePreferencesStore.setState({
    userId,
    preferences: previous.userId === userId ? previous.preferences : null,
    error: null,
  });
  const promise = (async () => {
    try {
      const preferences = await getPackagePreferences();
      if (version !== requestVersion || useAuthStore.getState().user?.id !== userId) return;
      usePackagePreferencesStore.setState({ preferences, error: null });
    } catch (error) {
      if (version !== requestVersion || useAuthStore.getState().user?.id !== userId) return;
      usePackagePreferencesStore.setState({
        error: extractErrorMessage(error, translateMessage(getActiveLocale(), "settingsAdmin.packageLoadFailed")),
      });
      throw error;
    }
  })().finally(() => {
    if (pending?.promise === promise) pending = null;
  });
  pending = { userId, promise };
  return promise;
}

export async function savePackagePreferences(hiddenPackageIds: PackageId[]): Promise<void> {
  const userId = useAuthStore.getState().user?.id;
  if (userId == null) throw new Error(translateMessage(getActiveLocale(), "auth.sessionChanged"));
  const preferences = await updatePackagePreferences({ hidden_package_ids: hiddenPackageIds });
  if (useAuthStore.getState().user?.id !== userId) return;
  requestVersion += 1;
  usePackagePreferencesStore.setState({ userId, preferences, error: null });
}

export function usePackagePreferences() {
  const userId = useAuthStore(state => state.user?.id ?? null);
  const state = usePackagePreferencesStore();
  useEffect(() => {
    const current = usePackagePreferencesStore.getState();
    if (userId == null) {
      if (current.userId != null) {
        requestVersion += 1;
        usePackagePreferencesStore.setState({ userId: null, preferences: null, error: null });
      }
      return;
    }
    if (current.userId === userId && (current.preferences != null || current.error != null)) return;
    // The layout displays the shared error once, rather than once per navigation consumer.
    void refreshPackagePreferences(userId).catch(() => undefined);
  }, [userId]);
  const preferences = state.userId === userId ? state.preferences : null;
  const isVisible = useCallback((packageId: PackageId) =>
    preferences != null && !preferences.hidden_package_ids.includes(packageId), [preferences]);
  const refresh = useCallback(() => {
    if (userId == null) return Promise.reject(new Error(translateMessage(getActiveLocale(), "auth.sessionChanged")));
    return refreshPackagePreferences(userId);
  }, [userId]);
  return {
    preferences,
    error: state.userId === userId ? state.error : null,
    isVisible,
    refresh,
  };
}
