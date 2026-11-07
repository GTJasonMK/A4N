import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import { useBlockerProceed } from "./useBlockerProceed";

it("allows only one continuation when automatic cleanup and discard both finish", () => {
  const advance = vi.fn();
  let proceed: () => void;
  function Guard() {
    proceed = useBlockerProceed({
      state: "blocked",
      proceed: advance,
      reset: vi.fn(),
      location: { pathname: "/next", search: "", hash: "", state: null, key: "next", unstable_mask: undefined },
    });
    return null;
  }
  renderToStaticMarkup(<Guard />);
  proceed!();
  proceed!();
  expect(advance).toHaveBeenCalledOnce();
});

it("does not continue a navigation that is no longer blocked", () => {
  let proceed: () => void;
  function Guard() {
    proceed = useBlockerProceed({ state: "unblocked", proceed: undefined, reset: undefined, location: undefined });
    return null;
  }
  renderToStaticMarkup(<Guard />);
  expect(() => proceed!()).not.toThrow();
});
