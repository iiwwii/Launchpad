import { create } from "zustand";
import type { AppConfig, Entry } from "../types";

interface State {
  config: AppConfig | null;
  currentDir: string;
  isRoot: boolean;
  entries: Entry[];
  loading: boolean;
  search: string;
  searchResults: Entry[] | null; // null=未搜索
  openedFolder: Entry | null; // 当前弹出的文件夹框（Mac 风格）
  folderClosing: boolean; // 文件夹框关闭中（同步首页 blur 收起）
  setConfig: (c: AppConfig) => void;
  setOpenedFolder: (e: Entry | null) => void;
  setFolderClosing: (b: boolean) => void;
  setDir: (dir: string, isRoot: boolean) => void;
  setEntries: (e: Entry[]) => void;
  setLoading: (b: boolean) => void;
  setSearch: (s: string) => void;
  setSearchResults: (r: Entry[] | null) => void;
}

export const useStore = create<State>((set) => ({
  config: null,
  currentDir: "",
  isRoot: true,
  entries: [],
  loading: false,
  search: "",
  searchResults: null,
  openedFolder: null,
  folderClosing: false,
  setConfig: (c) => set({ config: c }),
  setOpenedFolder: (e) => set({ openedFolder: e }),
  setFolderClosing: (b) => set({ folderClosing: b }),
  setDir: (dir, isRoot) => set({ currentDir: dir, isRoot }),
  setEntries: (e) => set({ entries: e }),
  setLoading: (b) => set({ loading: b }),
  setSearch: (s) => set({ search: s }),
  setSearchResults: (r) => set({ searchResults: r }),
}));
