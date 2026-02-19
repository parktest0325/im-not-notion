export const GLOBAL_FUNCTIONS = Symbol('globalFunctions');
export interface GlobalFunctions {
  refreshList: () => Promise<void>;
}
