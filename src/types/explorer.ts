// Mirrors Rust types in transfer_service.rs / fs_service.rs.
// Keep both sides in sync when changing.

export type ConflictPolicy = "overwrite" | "skip" | "rename";

export interface ConflictItem {
  name: string;
  is_dir: boolean;
  size: number;
}

export interface FsEntry {
  name: string;
  is_dir: boolean;
  size: number;
  modified: number | null;
}

export type TransferPhase =
  | "packing"
  | "uploading"
  | "extracting"
  | "downloading"
  | "cleanup"
  | "done"
  | "error";

export interface TransferProgress {
  id: string;
  phase: TransferPhase;
  current_bytes: number;
  total_bytes: number;
  files_done: number;
  files_total: number;
  current_file: string;
  error: string | null;
}
