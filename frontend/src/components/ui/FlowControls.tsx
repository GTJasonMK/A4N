import { ControlButton, Controls, useReactFlow, useStore, useStoreApi } from "@xyflow/react";
import { LockKeyhole, LockKeyholeOpen, Minus, Plus, Scan } from "lucide-react";

export function FlowControls({ showInteractive = true }: { showInteractive?: boolean }) {
  const { zoomIn, zoomOut, fitView } = useReactFlow();
  const store = useStoreApi();
  const interactive = useStore((state) => state.nodesDraggable || state.nodesConnectable || state.elementsSelectable);
  const minZoomReached = useStore((state) => state.transform[2] <= state.minZoom);
  const maxZoomReached = useStore((state) => state.transform[2] >= state.maxZoom);
  const labels = useStore((state) => state.ariaLabelConfig);
  const toggleInteraction = () => store.setState({
    nodesDraggable: !interactive,
    nodesConnectable: !interactive,
    elementsSelectable: !interactive,
  });

  return (
    <Controls className="a4n-flow-controls" showZoom={false} showFitView={false} showInteractive={false}>
      <ControlButton className="react-flow__controls-zoomin" disabled={maxZoomReached}
        title={labels["controls.zoomIn.ariaLabel"]} aria-label={labels["controls.zoomIn.ariaLabel"]} onClick={() => void zoomIn()}>
        <Plus size={16} aria-hidden="true" />
      </ControlButton>
      <ControlButton className="react-flow__controls-zoomout" disabled={minZoomReached}
        title={labels["controls.zoomOut.ariaLabel"]} aria-label={labels["controls.zoomOut.ariaLabel"]} onClick={() => void zoomOut()}>
        <Minus size={16} aria-hidden="true" />
      </ControlButton>
      <ControlButton className="react-flow__controls-fitview"
        title={labels["controls.fitView.ariaLabel"]} aria-label={labels["controls.fitView.ariaLabel"]} onClick={() => void fitView()}>
        <Scan size={16} aria-hidden="true" />
      </ControlButton>
      {showInteractive && (
        <ControlButton className="react-flow__controls-interactive" aria-pressed={!interactive}
          title={labels["controls.interactive.ariaLabel"]} aria-label={labels["controls.interactive.ariaLabel"]} onClick={toggleInteraction}>
          {interactive ? <LockKeyholeOpen size={16} aria-hidden="true" /> : <LockKeyhole size={16} aria-hidden="true" />}
        </ControlButton>
      )}
    </Controls>
  );
}
