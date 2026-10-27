<script lang="ts">
  // Édition d'une zone de texte avec TipTap, limitée à la mise en forme exportable en ODG.
  import { Editor } from "@tiptap/core";
  import Bold from "@tiptap/extension-bold";
  import Document from "@tiptap/extension-document";
  import Italic from "@tiptap/extension-italic";
  import Paragraph from "@tiptap/extension-paragraph";
  import Text from "@tiptap/extension-text";
  import TextAlign from "@tiptap/extension-text-align";
  import { Color, FontFamily, FontSize, TextStyle } from "@tiptap/extension-text-style";
  import Underline from "@tiptap/extension-underline";
  import { Placeholder, UndoRedo } from "@tiptap/extensions";
  import { onDestroy, onMount } from "svelte";
  import { editor as doc } from "../lib/docStore.svelte";
  import { richToTiptap, sanitizePlainText, tiptapToRich } from "../lib/richtext";
  import { textEditing } from "../lib/textEditing.svelte";
  import type { RichText, TextProps } from "../lib/types";

  let { props, ondone }: { props: TextProps; ondone: (content: RichText) => void } = $props();

  let host: HTMLDivElement;
  let editor: Editor | null = null;
  let finished = false;
  let stopFlush: (() => void) | null = null;

  function finish() {
    if (finished || !editor) return;
    finished = true;
    ondone(tiptapToRich(editor.getJSON(), props.style));
  }

  onMount(() => {
    editor = new Editor({
      element: host,
      extensions: [
        Document,
        Paragraph,
        Text,
        Bold,
        Italic,
        Underline,
        TextStyle,
        Color,
        FontFamily,
        FontSize,
        TextAlign.configure({ types: ["paragraph"], alignments: ["left", "center", "right", "justify"] }),
        UndoRedo,
        Placeholder.configure({ placeholder: props.prompt ?? "" }),
      ],
      content: richToTiptap(props.content),
      editorProps: {
        attributes: { class: "tiptap-host", spellcheck: "true", lang: "fr" },
        // Le collage et le dépôt ne gardent que le texte : pas de mise en forme impossible à exporter.
        handlePaste(view, event) {
          const text = event.clipboardData?.getData("text/plain");
          if (text == null) return false;
          view.dispatch(view.state.tr.insertText(sanitizePlainText(text)));
          return true;
        },
        handleDrop(view, event) {
          const text = event.dataTransfer?.getData("text/plain");
          if (!text) return false;
          event.preventDefault();
          const pos = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos ?? view.state.selection.from;
          view.dispatch(view.state.tr.insertText(sanitizePlainText(text), pos));
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
        // Un clic dans le panneau de droite (police, couleur...) ne termine pas l'édition.
        const next = event.relatedTarget as HTMLElement | null;
        if (next?.closest(".right")) return;
        finish();
      },
      // Différé : la fermeture de l'éditeur provoque une transaction pendant le rendu de Svelte.
      onSelectionUpdate: () => queueMicrotask(() => textEditing.tick++),
      onTransaction: () => queueMicrotask(() => textEditing.tick++),
    });
    textEditing.current = editor;
    // Enregistrer ou fermer pendant la saisie : le texte tapé est validé d'abord.
    stopFlush = doc.onFlush(finish);
    editor.commands.focus("end");
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
