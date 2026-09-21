import { useEffect, useRef, useState } from "react";

/** Workspace keeps visited pages mounted; only the visible page owns runtime requests. */
export function useRuntimeVisibility() {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const refresh = () => setVisible(document.visibilityState !== "hidden" && Boolean(ref.current) && !ref.current?.closest("[hidden]"));
    const observer = new MutationObserver(refresh);
    for (let element: HTMLElement | null = ref.current; element; element = element.parentElement) {
      observer.observe(element, { attributes: true, attributeFilter: ["hidden"] });
    }
    document.addEventListener("visibilitychange", refresh);
    refresh();
    return () => { observer.disconnect(); document.removeEventListener("visibilitychange", refresh); };
  }, []);
  return { ref, visible };
}
