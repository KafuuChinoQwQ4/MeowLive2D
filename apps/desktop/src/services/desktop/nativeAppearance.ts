export function shouldUseNativeGlass(nativeWindow: boolean, userAgent: string): boolean {
  return nativeWindow && !/linux/i.test(userAgent);
}
