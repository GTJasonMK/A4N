export function installTauriEventMock(internals) {
  const callbacks = new Map();
  const listeners = new Map();
  let nextCallbackId = 1;
  let nextEventId = 1;
  const invoke = internals.invoke;

  internals.transformCallback = (callback, once = false) => {
    const id = nextCallbackId++;
    callbacks.set(id, { callback, once });
    return id;
  };
  internals.unregisterCallback = (id) => callbacks.delete(id);
  internals.invoke = async (command, args = {}) => {
    if (command === "plugin:event|listen") {
      if (!callbacks.has(args.handler)) {
        throw new Error(`unknown event callback: ${args.handler}`);
      }
      const eventId = nextEventId++;
      listeners.set(eventId, {
        event: args.event,
        handlerId: args.handler,
        target: args.target,
      });
      return eventId;
    }
    if (command === "plugin:event|unlisten") {
      const listener = listeners.get(args.eventId);
      if (!listener || listener.event !== args.event) {
        throw new Error(`unknown event listener: ${args.eventId}`);
      }
      listeners.delete(args.eventId);
      callbacks.delete(listener.handlerId);
      return null;
    }
    return invoke(command, args);
  };

  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: (event, eventId) => {
      const listener = listeners.get(eventId);
      if (!listener || listener.event !== event) {
        throw new Error(`unknown event listener: ${eventId}`);
      }
      callbacks.delete(listener.handlerId);
    },
  };

  return {
    emit(event, payload) {
      for (const [eventId, listener] of listeners) {
        if (listener.event !== event) continue;
        const registration = callbacks.get(listener.handlerId);
        if (!registration) throw new Error(`unknown event callback: ${listener.handlerId}`);
        registration.callback({ event, id: eventId, payload });
        if (registration.once) {
          listeners.delete(eventId);
          callbacks.delete(listener.handlerId);
        }
      }
    },
  };
}
