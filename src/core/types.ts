/**
 * Core Data Models for CLIBM v0.1.0
 * Matches `/rust-clibm/src/models.rs` and `/rust-clibm/src/cas.rs`
 */

export type ObjectType = 'workspace' | 'project' | 'task' | 'board' | 'tree' | 'commit';

export const FORMAT_VERSION = '0.1.0';

/**
 * Specification 2.2.2: Immutable Object Envelope
 */
export interface CasObject<T = unknown> {
  Type: ObjectType;
  Version: string;
  Hash: string;
  Payload: T;
}

export type CoreInstallProfile = 'pure' | 'with-defaults';
export type ClibmLanguage = 'en' | 'pt-BR' | 'es';
export type McpTransportMode = 'both' | 'stdio' | 'http';

/**
 * Repository & machine installation configuration stored in `.clibm/config.json`.
 * `auto_commit` is enabled by default (`true`) so every individual modification
 * automatically creates an immutable BLAKE3 commit on the active branch.
 * `install_profile` defaults to `'pure'` (0 pre-installed plugins).
 */
export interface ClibmConfig {
  auto_commit: boolean;
  author_name: string;
  author_email: string;
  default_priority: TaskPriority;
  enforce_dependencies: boolean;
  enforce_wip_limits: boolean;
  cross_workspace_dependencies: boolean;
  github_repo_url: string;
  update_branch: string;
  install_profile: CoreInstallProfile;
  language: ClibmLanguage;
  install_path: string;
  mcp_transport: McpTransportMode;
}

export interface WorkspacePayload {
  id: string;
  name: string;
  description: string;
  created_at: string;
  properties: Record<string, string>;
  project_ids: string[];
}

export interface Participant {
  id: string;
  name: string;
  email: string;
  role: string;
}

export interface ProjectPayload {
  id: string;
  workspace_id: string;
  code: string;
  name: string;
  description: string;
  created_at: string;
  participants: Participant[];
  metadata: Record<string, string>;
  board_ids: string[];
}

export type TaskPriority = 'low' | 'medium' | 'high' | 'critical';

export interface TaskComment {
  id: string;
  author: string;
  timestamp: string;
  body: string;
}

export interface TaskPayload {
  id: string;
  project_id: string;
  board_id: string;
  column_id: string;
  title: string;
  description: string;
  priority: TaskPriority;
  assignee: string | null;
  due_date: string | null;
  created_at: string;
  updated_at: string;
  tags: string[];
  depends_on: string[];
  comments: TaskComment[];
  metadata: Record<string, string>;
}

export interface KanbanColumn {
  id: string;
  name: string;
  order: number;
  wip_limit: number | null;
  is_completed_state: boolean;
}

export interface WorkflowConfig {
  enforce_dependencies: boolean;
  enforce_wip_limits: boolean;
  require_assignee_in_progress: boolean;
}

export interface KanbanBoardPayload {
  id: string;
  project_id: string;
  name: string;
  columns: KanbanColumn[];
  workflow: WorkflowConfig;
}

export interface TreeSnapshotPayload {
  active_workspace_id: string;
  active_project_id: string;
  workspaces: Record<string, string>;
  projects: Record<string, string>;
  boards: Record<string, string>;
  tasks: Record<string, string>;
}

export interface CommitPayload {
  tree_hash: string;
  parent_hashes: string[];
  author: string;
  timestamp: string;
  message: string;
}

export interface ComputedIndex {
  tree_hash: string;
  by_project: Record<string, string[]>;
  by_column: Record<string, string[]>;
  by_priority: Record<string, string[]>;
  by_tag: Record<string, string[]>;
  by_assignee: Record<string, string[]>;
  blocks_tasks: Record<string, string[]>;
  dependency_cycles: string[][];
}

export interface EntityChange {
  entity_type: ObjectType;
  entity_id: string;
  change_kind: 'added' | 'modified' | 'deleted';
  old_hash: string | null;
  new_hash: string | null;
  details: string[];
}

export interface DiffReport {
  from_ref: string;
  to_ref: string;
  from_tree_hash: string;
  to_tree_hash: string;
  changes: EntityChange[];
}

export interface TaskHistoryEntry {
  commit_hash: string;
  commit_timestamp: string;
  commit_author: string;
  commit_message: string;
  task_object_hash: string;
  task_snapshot: TaskPayload;
}

export interface PluginManifest {
  name: string;
  version: string;
  description: string;
  enabled: boolean;
  hooks: string[];
  executable: string | null;
}

export interface TerminalHistoryEntry {
  id: string;
  command: string;
  output: string;
  status: 'ok' | 'error' | 'info';
  timestamp: string;
}

export interface ClibmRepositoryState {
  objects: Record<string, CasObject>;
  branches: Record<string, string>;
  head: string; // e.g., "ref: refs/heads/main" or raw commit hash
  stagedTreeHash: string;
  plugins: PluginManifest[];
  config: ClibmConfig;
}
