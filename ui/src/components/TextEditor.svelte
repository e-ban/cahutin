<script lang="ts">
  // Édition d'une zone de texte avec TipTap, limitée à la mise en forme exportable en ODG.
  import { Editor } from "@tiptap/core";
  import Bold from "@tiptap/extension-bold";
  import Italic from "@tiptap/extension-italic";
  import TextAlign from "@tiptap/extension-text-align";
  import { Color, FontFamily, FontSize, TextStyle } from "@tiptap/extension-text-style";
  import Underline from "@tiptap/extension-underline";
  import { Placeholder, UndoRedo } from "@tiptap/extensions";
  import { Slice } from "@tiptap/pm/model";
  import { onDestroy, onMount } from "svelte";
  import { editor as doc } from "../lib/docStore.svelte";
  import { plainText } from "../lib/fieldNode";
  import { cssFontFamily, fontFamilyName, richToTiptap, tiptapToRich } from "../lib/richtext";
  import { keepsEditing, textEditing } from "../lib/textEditing.svelte";
  import { insertLines, readable, textNodes } from "../lib/textNodes";
  import { copiedSlice, rememberCopy, withZoneStyle, type CopiedSlice } from "../lib/textClipboard";
  import type { RichText, TextProps } from "../lib/types";

  let { props, ondone }: { props: TextProps; ondone: (content: RichText) => void } = $props();

  // Police d'un passage : nom entre guillemets, avec Andika en secours, comme hors édition (TipTap l'écrit
  // nu, et un nom comme « Cursive 2 » est alors ignoré : le passage prenait la police de la zone).
  const QuotedFontFamily = FontFamily.extend({
    addGlobalAttributes() {
      return [
        {
          types: this.options.types,
          attributes: {
            fontFamily: {
              default: null,
              parseHTML: (el: HTMLElement) => fontFamilyName(el.style.fontFamily),
              renderHTML: (a: { fontFamily?: string | null }) => (a.fontFamily ? { style: `font-family: ${cssFontFamily(a.fontFamily)}, "Andika"` } : {}),
            },
          },
        },
      ];
    },
  });

  let host: HTMLDivElement;
  let editor: Editor | null = null;
  let finished = false;
  // Texte enregistré que l'éditeur ne sait pas lire : TipTap ouvre une zone vide, qui ne doit pas l'effacer.
  let unreadable = false;
  let stopFlush: (() => void) | null = null;

  function finish() {
    if (finished || !editor) return;
    finished = true;
    ondone(unreadable ? props.content : tiptapToRich(editor.getJSON(), props.style));
  }

  onMount(() => {
    const content = richToTiptap(props.content);
    editor = new Editor({
      element: host,
      extensions: [
        ...textNodes,
        Bold,
        Italic,
        Underline,
        TextStyle,
        Color,
        QuotedFontFamily,
        FontSize,
        TextAlign.configure({ types: ["paragraph"], alignments: ["left", "center", "right", "justify"] }),
        UndoRedo,
        Placeholder.configure({ placeholder: props.prompt ?? "" }),
      ],
      content,
      editorProps: {
        attributes: { class: "tiptap-host", spellcheck: "true", lang: "fr" },
        // Copie : le texte va au presse-papiers, le passage et sa mise en forme (avec le style de la zone) sont
        // retenus pour un collage dans une autre zone.
        clipboardTextSerializer(slice) {
          const text = plainText(slice.content);
          rememberCopy(text, withZoneStyle(slice.toJSON() as CopiedSlice, props.style));
          return text;
        },
        // Un passage copié dans Cahutin revient avec sa mise en forme ; ce qui vient d'ailleurs, et le dépôt, ne
        // gardent que le texte : pas de mise en forme impossible à exporter.
        handlePaste(view, event) {
          const text = event.clipboardData?.getData("text/plain");
          if (text == null) return false;
          const copied = copiedSlice(text);
          if (copied) {
            try {
              view.dispatch(view.state.tr.replaceSelection(Slice.fromJSON(view.state.schema, copied)).scrollIntoView());
              return true;
            } catch {
              // Passage illisible pour cet éditeur : collé en texte brut.
            }
          }
          view.dispatch(insertLines(view.state, text));
          return true;
        },
        handleDrop(view, event) {
          const text = event.dataTransfer?.getData("text/plain");
          if (!text) return false;
          event.preventDefault();
          const pos = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos ?? view.state.selection.from;
          view.dispatch(insertLines(view.state, text, pos));
          return true;
        },
        handleKeyDown(_view, event) {
          if (event.key === "Escape") {
            finish();
            return true;
          }
          return false;
        },
      },
      onBlur: ({ event }) => {
        if (keepsEditing(event.relatedTarget as HTMLElement | null)) return;
        finish();
      },
      // Différé : la fermeture de l'éditeur provoque une transaction pendant le rendu de Svelte.
      onSelectionUpdate: () => queueMicrotask(() => textEditing.tick++),
      onTransaction: () => queueMicrotask(() => textEditing.tick++),
    });
    unreadable = !readable(editor.schema, content);
    textEditing.current = editor;
    // Enregistrer ou fermer pendant la saisie : le texte tapé est validé d'abord.
    stopFlush = doc.onFlush(finish);
    // Texte modèle tout juste posé : sélectionné en entier, la première frappe le remplace.
    if (doc.selectTextOnEdit) editor.chain().focus().selectAll().run();
    else editor.commands.focus("end");
    doc.selectTextOnEdit = false;
  });

  onDestroy(() => {
    stopFlush?.();
    finish();
    if (textEditing.current === editor) textEditing.current = null;
    editor?.destroy();
  });
</script>

<div class="editor" bind:this={host} role="presentation" onpointerdown={(e) => e.stopPropagation()}></div>

<style>
  .editor,
  .editor :global(.tiptap-host) {
    width: 100%;
    height: 100%;
    outline: none;
    cursor: text;
  }
  .editor :global(p) {
    margin: 0;
  }
  .editor :global(p.is-editor-empty:first-child::before) {
    content: attr(data-placeholder);
    color: #9a9a9a;
    font-style: italic;
    float: left;
    height: 0;
    pointer-events: none;
  }
</style>
