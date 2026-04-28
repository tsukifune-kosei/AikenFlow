import {
  Background,
  Controls,
  Handle,
  MiniMap,
  Position,
  ReactFlow,
  type Edge,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import {
  AlertTriangle,
  Braces,
  Download,
  ExternalLink,
  FileCode2,
  GitBranch,
  Play,
  ShieldCheck,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  AikenFlowBundle,
  ArtefactGroup,
  GeneratedFinding,
  GeneratedArtefact,
  GraphState,
  GraphTransition,
  InvariantStatus,
  ProtocolSelection,
} from "../../src/shared/aikenflowTypes";
import type { ExtensionToWebview } from "../../src/shared/webviewMessages";
import { findArtefactLineForSelection } from "../../src/shared/artefactLocator";
import { postToExtension, vscodeApi } from "./vscodeApi";

type AppState = {
  bundle?: AikenFlowBundle;
  sourceUri?: string;
  isAnalysing: boolean;
  error?: string;
  selection?: ProtocolSelection;
  elapsedMs?: number;
};

type Finding = GeneratedFinding;

const nodeTypes = {
  state: StateNode,
  transition: TransitionNode,
  endpoint: EndpointNode,
};

export function App() {
  const [state, setState] = useState<AppState>(() => {
    const saved = vscodeApi()?.getState();
    return isAppState(saved) ? saved : { isAnalysing: false };
  });

  useEffect(() => {
    const listener = (event: MessageEvent<ExtensionToWebview>) => {
      const message = event.data;
      switch (message.type) {
        case "analysis.started":
          setState((current) => ({ ...current, isAnalysing: true, error: undefined }));
          break;
        case "analysis.completed":
          setState({
            bundle: message.bundle,
            sourceUri: message.sourceUri,
            isAnalysing: false,
            selection: firstSelection(message.bundle),
            elapsedMs: message.elapsedMs,
          });
          break;
        case "bundle.loaded":
          setState({
            bundle: message.bundle,
            sourceUri: message.sourceUri,
            isAnalysing: false,
            selection: firstSelection(message.bundle),
          });
          break;
        case "analysis.failed":
          setState((current) => ({
            ...current,
            isAnalysing: false,
            error: message.message,
          }));
          break;
        case "selection.changed":
          setState((current) => ({ ...current, selection: message.selection }));
          break;
      }
    };
    window.addEventListener("message", listener);
    return () => window.removeEventListener("message", listener);
  }, []);

  useEffect(() => {
    vscodeApi()?.setState(state);
  }, [state]);

  const findings = state.bundle?.findings ?? [];

  const select = useCallback((selection: ProtocolSelection) => {
    setState((current) => ({ ...current, selection }));
    postToExtension({ type: "selection.changed", selection });
  }, []);

  if (!state.bundle) {
    return (
      <main className="empty-shell">
        <section className="empty-panel">
          <div className="brand-row">
            <div className="brand-mark">AF</div>
            <div>
              <h1>AikenFlow Protocol Cockpit</h1>
              <p>Run real compiler analysis against the active protocol.yaml.</p>
            </div>
          </div>
          {state.error ? <div className="error-box">{state.error}</div> : null}
          <div className="actions-row">
            <button className="primary-action" onClick={() => postToExtension({ type: "command.analyse" })}>
              <Play size={16} />
              Analyse Protocol
            </button>
          </div>
        </section>
      </main>
    );
  }

  return (
    <main className="cockpit">
      <Header bundle={state.bundle} isAnalysing={state.isAnalysing} elapsedMs={state.elapsedMs} />
      {state.error ? <div className="error-strip">{state.error}</div> : null}
      <section className="main-grid">
        <GraphPanel bundle={state.bundle} selection={state.selection} onSelect={select} findings={findings} />
        <AssurancePanel bundle={state.bundle} findings={findings} selection={state.selection} onSelect={select} />
      </section>
      <section className="lower-grid">
        <Inspector bundle={state.bundle} selection={state.selection} findings={findings} />
        <Artefacts bundle={state.bundle} selection={state.selection} />
      </section>
    </main>
  );
}

function Header({ bundle, isAnalysing, elapsedMs }: { bundle: AikenFlowBundle; isAnalysing: boolean; elapsedMs?: number }) {
  const warnings = bundle.diagnostics.filter((diagnostic) => diagnostic.severity === "warning").length;
  const errors = bundle.diagnostics.filter((diagnostic) => diagnostic.severity === "error").length;
  return (
    <header className="header">
      <div className="brand-row">
        <div className="brand-mark">AF</div>
        <div>
          <h1>{bundle.protocol.name}</h1>
          <p>{bundle.protocol.description || "Protocol assurance workbench for Aiken builders."}</p>
        </div>
      </div>
      <div className="summary-strip">
        <Metric label="States" value={bundle.graph.states.length} />
        <Metric label="Transitions" value={bundle.graph.transitions.length} />
        <Metric label="Invariants" value={bundle.invariants.length} />
        <Metric label="Generated" value={bundle.metrics.generatedFiles} />
        <Metric label="Risk" value={bundle.metrics.riskLevel} tone={bundle.metrics.riskLevel} />
        <Metric label="Warnings" value={warnings + errors} tone={errors > 0 ? "high" : warnings > 0 ? "medium" : "low"} />
      </div>
      <div className="actions-row">
        <button onClick={() => postToExtension({ type: "command.analyse" })} disabled={isAnalysing}>
          <Play size={15} />
          {isAnalysing ? "Analysing" : "Analyse"}
        </button>
        <button onClick={() => postToExtension({ type: "command.generateArtefacts" })}>
          <Download size={15} />
          Generate
        </button>
        <button onClick={() => postToExtension({ type: "command.exportAudit" })}>
          <ShieldCheck size={15} />
          Audit
        </button>
        {elapsedMs ? <span className="elapsed">{elapsedMs}ms</span> : null}
      </div>
    </header>
  );
}

function Metric({ label, value, tone }: { label: string; value: string | number; tone?: string }) {
  return (
    <div className={`metric ${tone ? `tone-${tone}` : ""}`}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function GraphPanel({
  bundle,
  selection,
  findings,
  onSelect,
}: {
  bundle: AikenFlowBundle;
  selection?: ProtocolSelection;
  findings: Finding[];
  onSelect(selection: ProtocolSelection): void;
}) {
  const graph = useMemo(() => buildFlow(bundle, selection, findings), [bundle, selection, findings]);
  return (
    <section className="panel graph-panel">
      <PanelTitle icon={<GitBranch size={16} />} title="Protocol Graph" subtitle="State and transaction topology" />
      <div className="flow-shell">
        <ReactFlow
          nodes={graph.nodes}
          edges={graph.edges}
          nodeTypes={nodeTypes}
          fitView
          minZoom={0.35}
          maxZoom={1.5}
          onNodeClick={(_event, node) => {
            if (node.data.selection) onSelect(node.data.selection as ProtocolSelection);
          }}
        >
          <Background gap={18} size={1} color="rgba(126, 180, 255, 0.12)" />
          <Controls showInteractive={false} />
          <MiniMap pannable zoomable nodeColor={(node) => nodeColor(node)} />
        </ReactFlow>
      </div>
    </section>
  );
}

function AssurancePanel({
  bundle,
  findings,
  selection,
  onSelect,
}: {
  bundle: AikenFlowBundle;
  findings: Finding[];
  selection?: ProtocolSelection;
  onSelect(selection: ProtocolSelection): void;
}) {
  const covered = bundle.invariants.filter((item) => item.status === "covered").length;
  const partial = bundle.invariants.filter((item) => item.status === "partial").length;
  const missing = bundle.invariants.filter((item) => item.status === "missing").length;
  return (
    <section className="panel assurance-panel">
      <PanelTitle icon={<ShieldCheck size={16} />} title="Assurance" subtitle="Generated analysis, not a formal proof" />
      <div className="coverage-row">
        <div className="coverage-dial">
          <strong>{bundle.metrics.invariantCoverage}%</strong>
          <span>Invariant coverage</span>
        </div>
        <div className="coverage-copy">
          <p>
            {bundle.invariants.length} invariants detected, {covered} covered, {partial} partial, {missing} missing.
          </p>
          <p>Manual review required before production use.</p>
        </div>
      </div>
      <div className="matrix">
        <div className="matrix-head">
          <span>Invariant</span>
          <span>Status</span>
        </div>
        {bundle.invariants.map((invariant) => (
          <button
            key={invariant.name}
            className={`matrix-row ${selection?.kind === "invariant" && selection.id === invariant.name ? "selected" : ""}`}
            onClick={() => onSelect({ kind: "invariant", id: invariant.name })}
          >
            <span>
              <strong>{invariant.name}</strong>
              <em>{invariant.expression}</em>
            </span>
            <StatusPill value={invariant.status} />
          </button>
        ))}
      </div>
      <div className="findings-list">
        <h3>Top Findings</h3>
        {findings.length === 0 ? <p className="muted">No generated findings in this bundle.</p> : null}
        {findings.slice(0, 5).map((finding) => (
          <button
            key={finding.id}
            className={`finding finding-${finding.severity}`}
            onClick={() => {
              if (finding.transitionId) onSelect({ kind: "transition", id: finding.transitionId });
            }}
          >
            <AlertTriangle size={15} />
            <span>
              <strong>{finding.title}</strong>
              <em>{finding.detail}</em>
            </span>
          </button>
        ))}
      </div>
    </section>
  );
}

function Inspector({ bundle, selection, findings }: { bundle: AikenFlowBundle; selection?: ProtocolSelection; findings: Finding[] }) {
  const selectedState = selection?.kind === "state" ? bundle.graph.states.find((state) => state.id === selection.id) : undefined;
  const selectedTransition = selection?.kind === "transition"
    ? bundle.graph.transitions.find((transition) => transition.id === selection.id)
    : undefined;
  const selectedInvariant = selection?.kind === "invariant"
    ? bundle.invariants.find((invariant) => invariant.name === selection.id)
    : undefined;

  return (
    <section className="panel inspector-panel">
      <PanelTitle icon={<Braces size={16} />} title="Inspector" subtitle="Selected protocol topology" />
      {selectedState ? <StateInspector state={selectedState} transitions={bundle.graph.transitions} /> : null}
      {selectedTransition ? <TransitionInspector transition={selectedTransition} findings={findings} bundle={bundle} /> : null}
      {selectedInvariant ? <InvariantInspector invariant={selectedInvariant} /> : null}
      {!selectedState && !selectedTransition && !selectedInvariant ? <p className="muted">Select a graph node, invariant, or finding.</p> : null}
    </section>
  );
}

function StateInspector({ state, transitions }: { state: GraphState; transitions: GraphTransition[] }) {
  const consumedBy = transitions.filter((transition) => transition.from.includes(state.id)).map((transition) => transition.id);
  const producedBy = transitions.filter((transition) => transition.to.includes(state.id)).map((transition) => transition.id);
  const selection: ProtocolSelection = { kind: "state", id: state.id };
  return (
    <div className="inspector-content">
      <div className="inspector-heading">
        <h3>State: {state.label}</h3>
        <button onClick={() => postToExtension({ type: "reveal.source", target: selection })}>
          <ExternalLink size={14} />
          Source
        </button>
      </div>
      <InfoList title="Datum" items={state.datumFields.map((field) => `${field.name}: ${field.type}`)} />
      <InfoList title="Consumed by" items={consumedBy} />
      <InfoList title="Produced by" items={producedBy} />
      {state.terminal ? <StatusPill value="terminal" /> : null}
    </div>
  );
}

function TransitionInspector({ transition, findings, bundle }: { transition: GraphTransition; findings: Finding[]; bundle: AikenFlowBundle }) {
  const relatedFindings = findings.filter((finding) => finding.transitionId === transition.id);
  const relatedArtefacts = flattenArtefacts(bundle).filter((artefact) =>
    artefact.content.toLowerCase().includes(transition.id.toLowerCase()),
  );
  const selection: ProtocolSelection = { kind: "transition", id: transition.id };
  return (
    <div className="inspector-content">
      <div className="inspector-heading">
        <h3>Transition Inspector: {transition.label}</h3>
        <button onClick={() => postToExtension({ type: "reveal.source", target: selection })}>
          <ExternalLink size={14} />
          Source
        </button>
      </div>
      <div className="topology-card">
        <InfoList title="Consumes" items={transition.from.length > 0 ? transition.from : ["none"]} />
        <InfoList title="Produces" items={transition.to.length > 0 ? transition.to : ["terminal"]} />
        <InfoList title="Requires" items={transition.constraints.map((constraint) => constraint.label)} />
        <InfoList title="Generated" items={relatedArtefacts.slice(0, 4).map((artefact) => artefact.path)} />
      </div>
      {relatedFindings.map((finding) => (
        <div key={finding.id} className={`finding-note finding-${finding.severity}`}>{finding.title}</div>
      ))}
    </div>
  );
}

function InvariantInspector({ invariant }: { invariant: InvariantStatus }) {
  const selection: ProtocolSelection = { kind: "invariant", id: invariant.name };
  return (
    <div className="inspector-content">
      <div className="inspector-heading">
        <h3>Invariant: {invariant.name}</h3>
        <button onClick={() => postToExtension({ type: "reveal.source", target: selection })}>
          <ExternalLink size={14} />
          Source
        </button>
      </div>
      <p className="expression">{invariant.expression}</p>
      <StatusPill value={invariant.status} />
      <InfoList title="Mapped transitions" items={invariant.transitions.length > 0 ? invariant.transitions : ["not mapped"]} />
    </div>
  );
}

function Artefacts({ bundle, selection }: { bundle: AikenFlowBundle; selection?: ProtocolSelection }) {
  const [activeGroup, setActiveGroup] = useState<ArtefactGroup>("aiken");
  const files = bundle.artefacts[activeGroup];
  const suggested = useMemo(() => suggestArtefact(files, selection), [files, selection]);
  const [activePath, setActivePath] = useState<string | undefined>(suggested?.path ?? files[0]?.path);

  useEffect(() => {
    setActivePath(suggested?.path ?? files[0]?.path);
  }, [activeGroup, files, suggested]);

  const active = files.find((file) => file.path === activePath) ?? files[0];
  return (
    <section className="panel artefacts-panel">
      <PanelTitle icon={<FileCode2 size={16} />} title="Generated Artefacts" subtitle="Preview compiler output before writing files" />
      <div className="tabs">
        {Object.entries(bundle.artefacts).map(([group, groupFiles]) => (
          <button
            key={group}
            className={activeGroup === group ? "active" : ""}
            onClick={() => setActiveGroup(group as ArtefactGroup)}
          >
            {group} ({groupFiles.length})
          </button>
        ))}
      </div>
      <div className="artefact-body">
        <div className="file-list">
          {files.map((file) => (
            <button
              key={file.path}
              className={active?.path === file.path ? "selected" : ""}
              onClick={() => setActivePath(file.path)}
            >
              {file.path}
            </button>
          ))}
        </div>
        <div className="code-preview">
          {active ? (
            <>
              <div className="code-toolbar">
                <span>{active.path}</span>
                <button
                  onClick={() => postToExtension({
                    type: "reveal.artefact",
                    path: active.path,
                    line: findArtefactLineForSelection(active, selection),
                  })}
                >
                  <ExternalLink size={14} />
                  Open
                </button>
              </div>
              <pre><code>{active.content}</code></pre>
            </>
          ) : (
            <p className="muted">No generated files in this group.</p>
          )}
        </div>
      </div>
    </section>
  );
}

function PanelTitle({ icon, title, subtitle }: { icon: React.ReactNode; title: string; subtitle: string }) {
  return (
    <div className="panel-title">
      <div>{icon}</div>
      <span>
        <strong>{title}</strong>
        <em>{subtitle}</em>
      </span>
    </div>
  );
}

function InfoList({ title, items }: { title: string; items: string[] }) {
  return (
    <div className="info-list">
      <h4>{title}</h4>
      <ul>
        {items.map((item) => <li key={item}>{item}</li>)}
      </ul>
    </div>
  );
}

function StatusPill({ value }: { value: string }) {
  return <span className={`status-pill status-${value}`}>{value}</span>;
}

function StateNode(props: NodeProps) {
  const state = props.data.state as GraphState;
  return (
    <div className={`flow-node state-node ${props.selected ? "selected" : ""}`}>
      <Handle type="target" position={Position.Left} />
      <strong>{state.label}</strong>
      <span>UTxO State</span>
      <ul>
        {state.datumFields.slice(0, 4).map((field) => <li key={field.name}>{field.name}: {field.type}</li>)}
      </ul>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

function TransitionNode(props: NodeProps) {
  const transition = props.data.transition as GraphTransition;
  return (
    <div className={`flow-node transition-node risk-${transition.riskLevel} ${props.selected ? "selected" : ""}`}>
      <Handle type="target" position={Position.Left} />
      <strong>{transition.label}</strong>
      <span>Tx · {transition.riskLevel}</span>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

function EndpointNode(props: NodeProps) {
  return <div className="flow-node endpoint-node">{props.data.label as string}</div>;
}

function buildFlow(bundle: AikenFlowBundle, selection: ProtocolSelection | undefined, findings: Finding[]): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  const startId = "endpoint:start";
  const terminalId = "endpoint:terminal";
  nodes.push({
    id: startId,
    type: "endpoint",
    position: { x: 0, y: 60 },
    data: { label: "Start" },
    draggable: false,
  });
  nodes.push({
    id: terminalId,
    type: "endpoint",
    position: { x: 760, y: 60 },
    data: { label: "Terminal" },
    draggable: false,
  });

  bundle.graph.states.forEach((state, index) => {
    nodes.push({
      id: `state:${state.id}`,
      type: "state",
      position: { x: 250, y: 40 + index * 170 },
      data: { state, selection: { kind: "state", id: state.id } },
      selected: selection?.kind === "state" && selection.id === state.id,
    });
  });

  bundle.graph.transitions.forEach((transition, index) => {
    const hasFinding = findings.some((finding) => finding.transitionId === transition.id);
    nodes.push({
      id: `transition:${transition.id}`,
      type: "transition",
      position: { x: 560, y: 45 + index * 135 },
      data: { transition, selection: { kind: "transition", id: transition.id } },
      selected: selection?.kind === "transition" && selection.id === transition.id,
    });

    const sources = transition.from.length > 0 ? transition.from.map((id) => `state:${id}`) : [startId];
    const targets = transition.to.length > 0 ? transition.to.map((id) => `state:${id}`) : [terminalId];
    for (const source of sources) {
      edges.push(edge(`${source}->${transition.id}`, source, `transition:${transition.id}`, transition.riskLevel, hasFinding));
    }
    for (const target of targets) {
      edges.push(edge(`${transition.id}->${target}`, `transition:${transition.id}`, target, transition.riskLevel, hasFinding));
    }
  });

  return { nodes, edges };
}

function edge(id: string, source: string, target: string, risk: string, finding: boolean): Edge {
  const color = finding ? "#f87171" : risk === "low" ? "#22d3ee" : risk === "medium" ? "#fbbf24" : "#f87171";
  return {
    id,
    source,
    target,
    animated: finding,
    style: { stroke: color, strokeWidth: finding ? 2.5 : 1.6 },
  };
}

function nodeColor(node: Node): string {
  if (node.id.startsWith("transition:")) return "#8b5cf6";
  if (node.id.startsWith("state:")) return "#4da3ff";
  return "#64748b";
}

function flattenArtefacts(bundle: AikenFlowBundle): GeneratedArtefact[] {
  return Object.values(bundle.artefacts).flat();
}

function suggestArtefact(files: GeneratedArtefact[], selection?: ProtocolSelection): GeneratedArtefact | undefined {
  if (!selection || selection.kind !== "transition") return undefined;
  return files.find((file) => file.content.toLowerCase().includes(selection.id.toLowerCase()));
}

function firstSelection(bundle: AikenFlowBundle): ProtocolSelection | undefined {
  const transition = bundle.graph.transitions[0];
  if (transition) return { kind: "transition", id: transition.id };
  const state = bundle.graph.states[0];
  return state ? { kind: "state", id: state.id } : undefined;
}

function isAppState(value: unknown): value is AppState {
  return Boolean(value && typeof value === "object" && "isAnalysing" in value);
}
