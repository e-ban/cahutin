// Bibliothèque de modèles : la liste, et une page d'aperçu par modèle pour les miniatures.
import { deleteTemplate, instantiateTemplate, listTemplates, savePageAsTemplate } from "./api";
import { editor } from "./docStore.svelte";
import type { Page, TemplateInfo } from "./types";

class Templates {
  infos = $state.raw<TemplateInfo[]>([]);
  previews = $state.raw<Record<string, Page>>({});

  async refresh() {
    try {
      this.infos = await listTemplates();
    } catch {
      this.infos = [];
      return;
    }
    const previews: Record<string, Page> = {};
    for (const t of this.infos) {
      try {
        const inst = await instantiateTemplate(t.id);
        previews[t.id] = inst.page;
        editor.registerAssets(inst.assets);
      } catch {
        // Un modèle illisible n'a simplement pas de miniature.
      }
    }
    this.previews = previews;
  }

  /** Nouvelle page à partir du modèle ; `replace` remplace la page courante au lieu d'insérer après. */
  async addPage(id: string, replace = false) {
    const inst = await instantiateTemplate(id);
    const assets = Object.fromEntries(inst.assets.map((a) => [a.id, a]));
    const pages = [...editor.doc.pages];
    const at = replace ? editor.pageIndex : editor.pageIndex + 1;
    pages.splice(at, replace ? 1 : 0, inst.page);
    editor.commit({ ...editor.doc, pages, assets: { ...editor.doc.assets, ...assets } });
    editor.pageIndex = at;
    editor.selection = [];
  }

  async saveCurrentPage(name: string) {
    await savePageAsTemplate(name, editor.page);
    await this.refresh();
  }

  async remove(id: string) {
    await deleteTemplate(id);
    await this.refresh();
  }
}

export const templates = new Templates();
