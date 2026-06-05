<script lang="ts">
    import { onMount, onDestroy, untrack } from "svelte";
    import { EditorView, keymap, drawSelection, highlightActiveLine } from "@codemirror/view";
    import { EditorState } from "@codemirror/state";
    import {
        defaultKeymap,
        history,
        historyKeymap,
        indentWithTab,
    } from "@codemirror/commands";

    let path = $state("");
    let initialContent = $state("");
    let currentSha = $state("");
    let last = $state({ updated: "", committer: "", commitSha: "" });

    let editorElement: HTMLDivElement | undefined = $state();
    let view: EditorView | undefined = $state();
    let loading = $state(true);
    let loadError = $state("");

    type SaveState = "idle" | "saving" | "saved" | "conflict" | "error";
    let saveState: SaveState = $state<SaveState>("idle");
    let saveMsg = $state("");
    let dirty = $state(false);
    let lineCount = $state(1);
    let cursorLine = $state(1);
    let cursorCol = $state(1);

    const theme = EditorView.theme({
        "&": {
            height: "100%",
            background: "transparent",
            color: "#e8e8e6",
            fontSize: "13.5px",
            fontFamily: '"IBM Plex Mono", monospace',
        },
        ".cm-scroller": {
            overflow: "auto",
            lineHeight: "1.7",
            paddingBottom: "40vh",
        },
        ".cm-content": {
            padding: "24px 0 0 0",
            caretColor: "#c8a96e",
            maxWidth: "820px",
            margin: "0 auto",
        },
        ".cm-line": {
            padding: "0 40px",
        },
        ".cm-cursor": {
            borderLeftColor: "#c8a96e",
            borderLeftWidth: "2px",
        },
        ".cm-selectionBackground, ::selection": {
          background: "rgba(200, 169, 110, 0.15) !important",
        },
        "&.cm-focused .cm-selectionBackground": {
            background: "rgba(200, 169, 110, 0.2) !important",
        },
        ".cm-activeLine": {
            background: "rgba(255,255,255,0.025)",
        },
        ".cm-gutters": {
            display: "none",
        },
        ".cm-focused": {
            outline: "none",
        },
    });

    function updateCursor(state: EditorState) {
        const pos = state.selection.main.head;
        const line = state.doc.lineAt(pos);
        cursorLine = line.number;
        cursorCol = pos - line.from + 1;
        lineCount = state.doc.lines;
    }

    async function save() {
        if (!view || saveState === "saving") return;
        saveState = "saving";
        saveMsg = "";

        const content = view.state.doc.toString();

        try {
            const res = await fetch("/api/save", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ path, content, sha: currentSha }),
            });

            const json = await res.json();

            if (res.status === 409) {
                saveState = "conflict";
                saveMsg = "conflict — page was edited elsewhere";
                return;
            }

            if (!res.ok) {
                saveState = "error";
                saveMsg = json.error ?? `error ${res.status}`;
                return;
            }

            currentSha = json.sha;
            dirty = false;
            saveState = "saved";
            saveMsg = "saved";

            setTimeout(() => {
                if (saveState === "saved") saveState = "idle";
            }, 2500);
        } catch (e: any) {
        saveState = "error";
        saveMsg = e?.message ?? "network error";
        }
    }

    onMount(async () => {
        const params = new URLSearchParams(window.location.search);
        const pagePath = params.get("path");

        if (!pagePath) {
            loadError = "no path specified";
            loading = false;
            return;
        }

        path = pagePath;

        const pathEl = document.getElementById("header-path");
        if (pathEl) pathEl.textContent = pagePath;

        try {
            const res = await fetch(`/api/pages/${pagePath}`);

            if (res.status === 401) {
                window.location.href = "/signin";
                return;
            }

            if (!res.ok) {
                loadError = `failed to load page (${res.status})`;
                loading = false;
                return;
            }

            const data = await res.json();
            initialContent = data.content;
            currentSha = data.sha ?? "";
            last = data.last ?? { updated: "", committer: "", commitSha: "" };

            loading = false;

            if (!editorElement) return;

            const saveKeymap = keymap.of([
            {
                key: "Mod-s",
                preventDefault: true,
                run: () => { save(); return true; },
            },
            ]);

            const state = EditorState.create({
                doc: initialContent,
                extensions: [
                    history(),
                    drawSelection(),
                    highlightActiveLine(),
                    keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
                    saveKeymap,
                    theme,
                    EditorView.lineWrapping,
                    EditorView.updateListener.of((update) => {
                        if (update.docChanged) {
                            dirty = true;
                            if (saveState === "saved") saveState = "idle";
                        }
                        if (update.selectionSet || update.docChanged) {
                            updateCursor(update.state);
                        }
                    }),
                ],
            });

            view = new EditorView({ state, parent: editorElement });
            view.focus();
            updateCursor(state);
        } catch (e: any) {
            loadError = e?.message ?? "network error";
            loading = false;
        }
    });

    onDestroy(() => {
        view?.destroy();
    });

    let statusColor = $derived(
        saveState === "saving" ? "#7a7a78"
        : saveState === "saved" ? "#5a9e6f"
        : saveState === "conflict" ? "#c8a96e"
        : saveState === "error" ? "#c0504a"
        : dirty ? "#5a7a9e"
        : "#3a3a38"
    );

    let statusLabel = $derived(
        saveState === "saving" ? "saving…"
        : saveState === "saved" ? saveMsg
        : saveState === "conflict" ? saveMsg
        : saveState === "error" ? saveMsg
        : dirty ? "unsaved"
        : ""
    );
</script>

{#if loading}
  <div class="loading-state">
    <span>loading…</span>
  </div>
{:else if loadError}
  <div class="error-state">
    <p>could not load page</p>
    <code>{loadError}</code>
  </div>
{:else}
  <div class="editor-root">
    <div class="cm-host" bind:this={editorElement}></div>

    <footer>
      <div class="footer-left">
        <span class="lastMod-label"><a href={"https://github.com/lorearchive/law-content/commits/main/" + path} class="a-no-style">Last modified on {last.updated ? new Date(last.updated).toLocaleDateString() : "?"} by {last.committer || "?"} @ {(last.commitSha || "").slice(0, 7) || "?"}</a></span>
      </div>
      <div class="footer-right">
        {#if statusLabel}
          <span class="status-msg" style:color={statusColor}>{statusLabel}</span>
        {/if}
        <span class="cursor-pos">
          {cursorLine}:{cursorCol}
        </span>
        <span class="line-count">{lineCount}L</span>
        <button
          class="save-btn"
          class:saving={saveState === "saving"}
          onclick={save}
          disabled={saveState === "saving"}
          title="Save (⌘S / Ctrl+S)"
        >
          {saveState === "saving" ? "saving…" : "save"}
        </button>
      </div>
    </footer>
  </div>
{/if}

<style>
  .editor-root {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .cm-host {
    flex: 1;
    overflow: hidden;
  }

  :global(.cm-host .cm-editor) {
    height: 100%;
  }

  .loading-state, .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 12px;
    color: var(--text-secondary);
    font-family: var(--font-ui);
  }

  .error-state code {
    font-family: var(--font-code);
    font-size: 12px;
    color: var(--danger);
    background: rgba(192, 80, 74, 0.1);
    padding: 4px 10px;
    border-radius: 4px;
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 32px;
    border-top: 1px solid #2a2a2e;
    background: #18181b;
    padding: 0 16px;
    flex-shrink: 0;
    user-select: none;
  }

  .footer-left,
  .footer-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .lastMod-label {
    font-family: "IBM Plex Mono", monospace;
    font-size: 11px;
    color: #4a4a48;
    letter-spacing: 0.02em;
  }

  .status-msg {
    font-family: "IBM Plex Mono", monospace;
    font-size: 11px;
    transition: color 0.2s;
  }

  .cursor-pos,
  .line-count {
    font-family: "IBM Plex Mono", monospace;
    font-size: 11px;
    color: #4a4a48;
  }

  .save-btn {
    font-family: "IBM Plex Sans", sans-serif;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0 10px;
    height: 22px;
    background: transparent;
    color: #c8a96e;
    border: 1px solid #4a3d28;
    border-radius: 3px;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, opacity 0.15s;
  }

  .save-btn:hover:not(:disabled) {
    background: rgba(200, 169, 110, 0.1);
    border-color: #c8a96e;
  }

  .save-btn:active:not(:disabled) {
    background: rgba(200, 169, 110, 0.18);
  }

  .save-btn:disabled,
  .save-btn.saving {
    opacity: 0.45;
    cursor: default;
  }

  .lastMod-label:hover {
    text-decoration: underline;
  }
</style>
