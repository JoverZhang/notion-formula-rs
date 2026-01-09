export {};

declare global {
  interface ObjectConstructor {
    keys<T extends object>(o: T): Array<Extract<keyof T, string>>;
  }
}
