import { useEffect, useState } from "react";
import { pageFromHash, type WorkspacePage } from "./navigation";

export function useWorkspaceNavigation() {
  const [active, setActive] = useState(() => pageFromHash(window.location.hash));
  const [visited, setVisited] = useState<Set<WorkspacePage>>(() => new Set([pageFromHash(window.location.hash)]));
  function accept(page: WorkspacePage) {
    setActive(page);
    setVisited(current => current.has(page) ? current : new Set([...current, page]));
  }
  useEffect(() => {
    const update = () => accept(pageFromHash(window.location.hash));
    window.addEventListener("hashchange", update);
    window.addEventListener("popstate", update);
    // The hash can change after rendering but before this effect subscribes.
    update();
    return () => { window.removeEventListener("hashchange", update); window.removeEventListener("popstate", update); };
  }, []);
  function navigate(page: WorkspacePage) {
    if (window.location.hash !== `#${page}`) window.history.pushState(null, "", `#${page}`);
    accept(page);
  }
  return { active, visited, navigate };
}
