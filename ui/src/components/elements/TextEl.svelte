<script lang="ts">
  import { TEXT_PADDING_MM } from "../../lib/richtext";
  import { mmToPx, ptToPx } from "../../lib/units";
  import type { Align, Element, Run, TextProps } from "../../lib/types";

  let { element, props, zoom }: { element: Element; props: TextProps; zoom: number } = $props();

  const blank = $derived(props.content.paragraphs.every((p) => p.runs.every((r) => r.text.trim() === "")));
  const cssAlign = (a: Align) => ({ Left: "left", Center: "center", Right: "right", Justify: "justify" })[a];

  function runStyle(r: Run): string {
    const s = props.style;
    return [
      `font-family: "${r.font ?? s.font}"`,
      `font-size: ${ptToPx(r.size_pt ?? s.size_pt, zoom)}px`,
      `color: ${r.color ?? s.color}`,
      r.bold ? "font-weight: 700" : "",
      r.italic ? "font-style: italic" : "",
      r.underline ? "text-decoration: underline" : "",
    ]
      .filter(Boolean)
      .join("; ");
  }
</script>

<div
  class="text"
  style:padding="{mmToPx(TEXT_PADDING_MM, zoom)}px"
  style:font-family={`"${props.style.font}"`}
  style:font-size="{ptToPx(props.style.size_pt, zoom)}px"
  style:color={props.style.color}
>
  {#if blank}
    {#if props.prompt}
      <p class="prompt zone-empty" style:text-align={cssAlign(props.style.align)}>{props.prompt}</p>
    {/if}
  {:else}
    {#each props.content.paragraphs as p}
      <p style:text-align={cssAlign(p.align ?? props.style.align)}>
        {#each p.runs as r}<span style={runStyle(r)}>{r.text}</span>{/each}{#if p.runs.every((r) => r.text === "")}<br />{/if}
      </p>
    {/each}
  {/if}
</div>

<style>
  .text {
    position: absolute;
    inset: 0;
    overflow: hidden;
    line-height: normal;
    white-space: pre-wrap;
    overflow-wrap: break-word;
    box-sizing: border-box;
  }
  p {
    margin: 0;
  }
  .prompt {
    color: #9a9a9a;
    font-style: italic;
  }
</style>
