import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Analysis, Asset, Destination, ExportResult, GateReport, Layout, MixPreview, Project, RenderOutcome, Summary, Voice,
} from "./types";

/** Tauri rejects with a string for plain errors and with an object for structured ones. */
export function messageOf(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object") {
    const o = e as { message?: unknown };
    if (typeof o.message === "string") return o.message;
  }
  return "Something went wrong.";
}

export const api = {
  info: () => invoke<{ version: string; projects_root: string }>("app_info"),
  listProjects: () => invoke<Summary[]>("list_projects"),
  createProject: (name: string, sample: boolean) => invoke<Project>("create_project", { name, sample }),
  loadProject: (id: string) => invoke<Project>("load_project", { id }),
  saveProject: (project: Project) => invoke<void>("save_project", { project }),
  deleteProject: (id: string) => invoke<void>("delete_project", { id }),
  projectDir: (id: string) => invoke<string>("project_dir", { id }),
  voices: () => invoke<Voice[]>("list_voices"),
  previewVoice: (voiceId: string, text: string) => invoke<{ path: string }>("preview_voice", { voiceId, text }),
  analyze: (project: Project) => invoke<Analysis>("analyze_project", { project }),
  renderLines: (project: Project, keys: string[]) => invoke<RenderOutcome>("render_lines", { project, keys }),
  waveform: (projectId: string, file: string, bins: number) => invoke<number[]>("waveform", { projectId, file, bins }),
  layout: (project: Project) => invoke<Layout>("compute_layout", { project }),
  destinations: () => invoke<Destination[]>("destinations"),
  mixPreview: (project: Project) => invoke<MixPreview>("mix_preview", { project }),
  checkExport: (project: Project) => invoke<GateReport>("check_export", { project }),
  exportProject: (project: Project, dest: string, acknowledged: boolean) =>
    invoke<ExportResult>("export_project", { project, dest, acknowledged }),
  importAsset: (projectId: string, path: string, kind: "music" | "sfx") =>
    invoke<Asset>("import_asset", { projectId, path, kind }),
  setApiKey: (provider: string, key: string) => invoke<void>("set_api_key", { args: { provider, key } }),
  testConnection: (provider: string) => invoke<string>("test_connection", { provider }),
  hasApiKey: (provider: string) => invoke<boolean>("has_api_key", { provider }),
  deleteApiKey: (provider: string) => invoke<void>("delete_api_key", { provider }),
  onRenderProgress: (fn: (p: { key: string; status: string }) => void) =>
    listen<{ key: string; status: string }>("render-progress", (e) => fn(e.payload)),
};

export function audioUrl(absolutePath: string, bust?: string): string {
  const u = convertFileSrc(absolutePath);
  return bust ? `${u}${u.includes("?") ? "&" : "?"}v=${encodeURIComponent(bust)}` : u;
}

export function joinPath(dir: string, rel: string): string {
  return `${dir.replace(/[\\/]+$/, "")}\\${rel.replace(/\//g, "\\")}`;
}
