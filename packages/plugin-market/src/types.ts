export interface PluginEntry {
  name: string;
  repo: string;
  description: string;
  category: string;
}

export interface PluginPreview {
  added: string[];
  removed: string[];
  patch_diff: string;
}
