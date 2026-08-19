window.__ModuleLoader__.load({
  id: "@dsh-desktop/dsh-codex-usage",
  factory: (require) => {
    const module = { exports: {} };
    const exports = module.exports;
    const React = require("react");
    const h = React.createElement;

    const CSS = `
      .dcu-action{position:relative;display:flex;align-items:center;gap:8px;min-width:0;height:32px;padding:0 8px;border:0;border-radius:8px;background:transparent;color:var(--dsw-alias-label-secondary);font:inherit;cursor:pointer}
      .dcu-action:hover{background:var(--dsw-alias-interactive-bg-hover);color:var(--dsw-alias-label-primary)}
      .dcu-ring{--p:0%;display:grid;place-items:center;width:24px;height:24px;flex:none;border-radius:50%;background:conic-gradient(var(--dsw-alias-brand-primary,#4f7cff) var(--p),var(--dsw-alias-border-l2,#d8dbe2) 0)}
      .dcu-ring:after{content:"";width:17px;height:17px;border-radius:50%;background:var(--dsw-alias-bg-layer-1,#fff)}
      .dcu-actionText{white-space:nowrap;font-size:12px}.dcu-dot{width:7px;height:7px;border-radius:50%;background:var(--dsw-alias-state-success-primary,#20a464)}
      .dcu-dot.off{background:var(--dsw-alias-label-dimmed,#999)}
      .dcu-pop{position:fixed;z-index:10000;left:16px;bottom:64px;width:min(360px,calc(100vw - 32px));padding:14px;border:1px solid var(--dsw-alias-border-l2);border-radius:14px;background:var(--dsw-alias-bg-layer-1,#fff);color:var(--dsw-alias-label-primary);box-shadow:0 16px 48px #0003;text-align:left}
      .dcu-popHead,.dcu-head,.dcu-row{display:flex;align-items:center;justify-content:space-between;gap:12px}.dcu-popHead{margin-bottom:12px}.dcu-popHead strong{font-size:14px}.dcu-close{border:0;background:transparent;color:inherit;font-size:20px;cursor:pointer}
      .dcu-section{box-sizing:border-box;width:100%;min-width:0;max-width:760px;overflow-x:clip;color:var(--dsw-alias-label-primary);display:flex;flex-direction:column;gap:18px}.dcu-title{margin:0;font-size:18px}.dcu-sub{margin:3px 0 0;color:var(--dsw-alias-label-tertiary);font-size:13px}.dcu-refresh{height:32px;padding:0 12px;border:1px solid var(--dsw-alias-border-l2);border-radius:16px;background:transparent;color:inherit;cursor:pointer}.dcu-refresh:hover{background:var(--dsw-alias-interactive-bg-hover)}
      .dcu-grid{display:grid;min-width:0;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:10px}.dcu-grid>*{min-width:0}.dcu-card{box-sizing:border-box;min-width:0;max-width:100%;overflow:hidden;border:1px solid var(--dsw-alias-border-l2);border-radius:12px;padding:13px;background:var(--dsw-alias-bg-module-platform,transparent)}.dcu-card h3{margin:0 0 10px;font-size:13px;font-weight:600}.dcu-row{font-size:12px;line-height:21px}.dcu-muted{color:var(--dsw-alias-label-tertiary)}
      .dcu-control{display:flex;flex-direction:column;gap:12px;padding:14px;border:1px solid var(--dsw-alias-border-l2);border-radius:14px;background:var(--dsw-alias-bg-module-platform,transparent)}.dcu-controlHead{display:flex;align-items:flex-start;justify-content:space-between;gap:12px}.dcu-controlHead h3{margin:0 0 3px;font-size:15px}.dcu-backends{display:grid;grid-template-columns:repeat(auto-fit,minmax(250px,1fr));gap:10px}.dcu-backend{display:flex;flex-direction:column;gap:10px;padding:12px;border:1px solid var(--dsw-alias-border-l2);border-radius:11px}.dcu-backend.selected{border-color:var(--dsw-alias-brand-primary,#4f7cff);box-shadow:inset 0 0 0 1px var(--dsw-alias-brand-primary,#4f7cff)}.dcu-backendTitle{display:flex;align-items:center;justify-content:space-between;gap:10px}.dcu-backendTitle strong{font-size:13px}.dcu-statusGrid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}.dcu-statusGrid>div{min-width:0;padding:7px 8px;border-radius:8px;background:var(--dsw-alias-bg-layer-1,#fff)}.dcu-statusGrid small{display:block;color:var(--dsw-alias-label-tertiary)}.dcu-statusGrid span{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:12px}.dcu-good{color:var(--dsw-alias-state-success-primary,#20a464)}.dcu-phase{display:flex;align-items:center;gap:8px;font-size:12px}.dcu-spinner{width:13px;height:13px;border:2px solid var(--dsw-alias-border-l2);border-top-color:var(--dsw-alias-brand-primary,#4f7cff);border-radius:50%;animation:dcu-spin .8s linear infinite}@keyframes dcu-spin{to{transform:rotate(360deg)}}.dcu-actions{display:flex;flex-wrap:wrap;gap:8px}.dcu-version{padding:2px 7px;border-radius:10px;background:var(--dsw-alias-interactive-bg-hover);font-size:11px}
      .dcu-bar{height:7px;margin:7px 0 5px;border-radius:5px;background:var(--dsw-alias-border-l2);overflow:hidden}.dcu-bar>i{display:block;height:100%;border-radius:inherit;background:var(--dsw-alias-brand-primary,#4f7cff)}.dcu-bar.warn>i{background:var(--dsw-alias-state-warn-label,#d28400)}.dcu-error{padding:10px 12px;border-radius:10px;background:var(--dsw-alias-interactive-bg-hover-danger,#fce8e8);color:var(--dsw-alias-state-error-primary,#c33);font-size:12px}.dcu-pill{padding:2px 7px;border-radius:10px;background:var(--dsw-alias-interactive-bg-hover);font-size:11px}.dcu-history{box-sizing:border-box;display:flex;align-items:flex-end;gap:1px;width:100%;min-width:0;max-width:100%;height:64px;margin-top:8px;overflow:hidden}.dcu-history i{flex:1 1 0;min-width:0;border-radius:2px 2px 0 0;background:var(--dsw-alias-brand-primary,#4f7cff);opacity:.75}.dcu-threads{display:flex;min-width:0;flex-direction:column;gap:7px}.dcu-thread{min-width:0;padding-top:7px;border-top:1px solid var(--dsw-alias-border-l2)}
      .dcu-handoff{height:26px;padding:0 10px;border:1px solid color-mix(in srgb,var(--dsw-alias-brand-primary,#4f7cff) 45%,transparent);border-radius:13px;background:color-mix(in srgb,var(--dsw-alias-brand-primary,#4f7cff) 10%,transparent);color:var(--dsw-alias-label-primary);font:inherit;font-size:12px;cursor:pointer}.dcu-handoff:hover{background:color-mix(in srgb,var(--dsw-alias-brand-primary,#4f7cff) 18%,transparent)}.dcu-handoff.live:before{content:"";display:inline-block;width:6px;height:6px;margin-right:6px;border-radius:50%;background:var(--dsw-alias-state-success-primary,#20a464)}
      .dcu-drawerBackdrop{position:fixed;z-index:19990;inset:0;background:#0006}.dcu-drawer{position:fixed;z-index:20000;top:0;right:0;display:flex;flex-direction:column;width:min(620px,100vw);height:100vh;border-left:1px solid var(--dsw-alias-border-l2);background:var(--dsw-alias-bg-layer-1,#fff);color:var(--dsw-alias-label-primary);box-shadow:-18px 0 60px #0004}[data-slot="conversation"]:has(.dcu-codexShell)>*{position:relative;overflow:hidden}.dcu-codexShell{position:absolute;z-index:20;inset:0;display:flex;min-width:0;min-height:0;flex-direction:column;background:var(--dsw-alias-bg-base,#fff);color:var(--dsw-alias-label-primary)}.dcu-codexComposerSeat{box-sizing:border-box;flex:none;width:100%;padding:12px max(16px,calc((100% - 780px)/2)) 20px;background:linear-gradient(180deg,transparent 0,var(--dsw-alias-bg-base,#fff) 24px)}.dcu-sessionView{display:flex;flex:1;min-width:0;min-height:0;flex-direction:column;background:var(--dsw-alias-bg-layer-1,#fff);color:var(--dsw-alias-label-primary)}.dcu-drawerHead{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:14px 16px;border-bottom:1px solid var(--dsw-alias-border-l2)}.dcu-drawerTitle{min-width:0}.dcu-drawerTitle strong{display:block}.dcu-drawerTitle small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--dsw-alias-label-tertiary)}.dcu-drawerActions{display:flex;gap:7px}.dcu-iconBtn,.dcu-secondary,.dcu-primary,.dcu-danger{height:32px;padding:0 12px;border:1px solid var(--dsw-alias-border-l2);border-radius:8px;background:transparent;color:inherit;font:inherit;cursor:pointer}.dcu-iconBtn{width:32px;padding:0;font-size:20px}.dcu-primary{border-color:transparent;background:var(--dsw-alias-brand-primary,#4f7cff);color:#fff}.dcu-danger{color:var(--dsw-alias-state-error-primary,#c33)}.dcu-iconBtn:disabled,.dcu-secondary:disabled,.dcu-primary:disabled,.dcu-danger:disabled{opacity:.45;cursor:not-allowed}
      .dcu-chat{flex:1;min-height:0;overflow:auto;padding:18px;display:flex;flex-direction:column;gap:12px}.dcu-empty{margin:auto;max-width:400px;text-align:center;color:var(--dsw-alias-label-tertiary)}.dcu-msg{max-width:92%;padding:11px 13px;border-radius:13px;white-space:pre-wrap;overflow-wrap:anywhere;font-size:13px;line-height:1.55}.dcu-msg.user{align-self:flex-end;background:var(--dsw-alias-brand-primary,#4f7cff);color:#fff}.dcu-msg.assistant{align-self:flex-start;border:1px solid var(--dsw-alias-border-l2);background:var(--dsw-alias-bg-module-platform,transparent)}.dcu-msg.plan{align-self:stretch;max-width:none;border:1px solid color-mix(in srgb,var(--dsw-alias-brand-primary,#4f7cff) 35%,var(--dsw-alias-border-l2));background:color-mix(in srgb,var(--dsw-alias-brand-primary,#4f7cff) 6%,transparent)}.dcu-planLabel{display:block;margin-bottom:6px;color:var(--dsw-alias-label-tertiary);font-size:11px;font-weight:600}.dcu-msg.live:after{content:"▋";animation:dcu-blink 1s step-end infinite}@keyframes dcu-blink{50%{opacity:0}}.dcu-activity{padding:8px 10px;border-left:3px solid var(--dsw-alias-border-l2);color:var(--dsw-alias-label-tertiary);font-size:11px;white-space:pre-wrap;overflow-wrap:anywhere}.dcu-approval{padding:12px;border:1px solid var(--dsw-alias-state-warn-label,#d28400);border-radius:12px;background:color-mix(in srgb,var(--dsw-alias-state-warn-label,#d28400) 8%,transparent);font-size:12px}.dcu-approval pre{max-height:120px;overflow:auto;white-space:pre-wrap}.dcu-approvalActions{display:flex;gap:7px;margin-top:9px}
      .dcu-compose{padding:12px 16px 16px;border-top:1px solid var(--dsw-alias-border-l2)}.dcu-compose textarea{box-sizing:border-box;width:100%;min-height:86px;max-height:220px;resize:vertical;padding:10px 12px;border:1px solid var(--dsw-alias-border-l2);border-radius:10px;background:var(--dsw-alias-bg-module-platform,transparent);color:inherit;font:inherit;line-height:1.5}.dcu-composeFoot,.dcu-composeTools{display:flex;align-items:center;gap:8px}.dcu-composeFoot{justify-content:space-between;margin-top:8px}.dcu-composeFoot small{color:var(--dsw-alias-label-tertiary)}.dcu-composeButtons{display:flex;gap:8px}
      .dcu-mainComposer{box-sizing:border-box;width:100%;min-width:0;border:1px solid var(--dsw-alias-border-l2);border-radius:18px;background:var(--dsw-alias-bg-layer-1,#fff);color:var(--dsw-alias-label-primary);box-shadow:0 8px 26px #0000000d;overflow:hidden}.dcu-mainComposer textarea{box-sizing:border-box;display:block;width:100%;min-height:82px;max-height:240px;padding:16px 18px 8px;border:0;outline:0;resize:vertical;background:transparent;color:inherit;font:inherit;font-size:14px;line-height:1.5}.dcu-mainFoot{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:8px 10px 10px}.dcu-mainTools{display:flex;align-items:center;justify-content:flex-end;gap:6px;min-width:0;margin-left:auto}.dcu-models{display:flex;align-items:center;justify-content:flex-end;gap:4px;min-width:0}.dcu-select{min-width:0;max-width:250px;height:30px;padding:0 25px 0 8px;border:0;border-radius:8px;background:transparent;color:inherit;font:inherit;font-size:12px;text-overflow:ellipsis;cursor:pointer}.dcu-select:hover{background:var(--dsw-alias-interactive-bg-hover)}.dcu-select:disabled{opacity:.55;cursor:not-allowed}.dcu-send{display:grid;place-items:center;flex:none;width:36px;height:36px;border:0;border-radius:50%;background:var(--dsw-alias-brand-primary,#4f7cff);color:#fff;font-size:20px;line-height:1;cursor:pointer}.dcu-send:disabled{opacity:.4;cursor:not-allowed}.dcu-composerError{padding:0 14px 10px;color:var(--dsw-alias-state-error-primary,#c33);font-size:11px;overflow-wrap:anywhere}
      .dcu-fileInput{display:none}.dcu-add{display:grid;place-items:center;flex:none;width:30px;height:30px;padding:0;border:0;border-radius:50%;background:var(--dsw-alias-interactive-bg-default,transparent);color:var(--dsw-alias-label-secondary);font:inherit;font-size:22px;font-weight:300;line-height:1;cursor:pointer}.dcu-add:hover{background:var(--dsw-alias-interactive-bg-hover);color:var(--dsw-alias-label-primary)}.dcu-add:disabled{opacity:.4;cursor:not-allowed}.dcu-attachments{box-sizing:border-box;display:flex;align-items:center;gap:6px;width:100%;min-width:0;padding:2px 12px 4px;overflow-x:auto}.dcu-compose>.dcu-attachments{padding:8px 0 0}.dcu-attachment{display:flex;align-items:center;gap:5px;min-width:0;max-width:220px;height:28px;padding:0 5px 0 9px;border:1px solid var(--dsw-alias-border-l2);border-radius:8px;background:var(--dsw-alias-bg-module-platform,transparent);font-size:11px}.dcu-attachment span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.dcu-attachment button{display:grid;place-items:center;flex:none;width:20px;height:20px;padding:0;border:0;border-radius:50%;background:transparent;color:var(--dsw-alias-label-tertiary);font:inherit;font-size:15px;cursor:pointer}.dcu-attachment button:hover{background:var(--dsw-alias-interactive-bg-hover);color:var(--dsw-alias-label-primary)}
      .dcu-modeRow{box-sizing:border-box;display:flex;align-items:center;width:100%;max-width:780px;margin:0 auto;padding:0 8px 4px}.dcu-modeSeat{position:relative;display:inline-flex;align-items:center;max-width:min(100%,240px);min-height:28px;border-radius:16px;color:var(--dsw-alias-label-primary)}.dcu-modeSeat:hover{background:var(--dsw-alias-interactive-bg-hover)}.dcu-modeSeat:before{content:"◇";position:absolute;left:8px;z-index:1;font-size:14px;pointer-events:none}.dcu-modeSeat:after{content:"⌄";position:absolute;right:7px;color:var(--dsw-alias-label-caption);font-size:12px;pointer-events:none}.dcu-modeSeat select{appearance:none;max-width:240px;height:28px;padding:0 24px 0 28px;border:0;border-radius:16px;background:transparent;color:inherit;font:inherit;font-size:13px;font-weight:500;line-height:20px;cursor:pointer;text-overflow:ellipsis}.dcu-modeSeat select:disabled{cursor:default;color:var(--dsw-alias-label-quaternary)}
      .dcu-goalDock{box-sizing:border-box;flex:none;width:100%;padding:0 max(16px,calc((100% - 780px)/2)) 6px}.dcu-goalBar{display:flex;align-items:center;gap:7px;min-width:0;min-height:34px;padding:0 10px;border:1px solid var(--dsw-alias-border-l2);border-radius:10px;background:var(--dsw-alias-bg-module-platform,transparent);font-size:12px}.dcu-goalGlyph{color:var(--dsw-alias-brand-primary,#4f7cff);font-size:15px}.dcu-goalLabel{flex:none;color:var(--dsw-alias-label-secondary);font-weight:600}.dcu-goalObjective{min-width:0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.dcu-goalMeta{flex:none;color:var(--dsw-alias-label-tertiary);font-size:11px}.dcu-goalActions{display:flex;align-items:center;gap:2px;margin-left:auto}.dcu-goalIcon{display:grid;place-items:center;width:26px;height:26px;padding:0;border:0;border-radius:7px;background:transparent;color:var(--dsw-alias-label-secondary);font:inherit;cursor:pointer}.dcu-goalIcon:hover:not(:disabled){background:var(--dsw-alias-interactive-bg-hover);color:var(--dsw-alias-label-primary)}.dcu-goalIcon:disabled{opacity:.4;cursor:default}.dcu-goalCreate{height:28px;padding:0 9px;border:0;border-radius:14px;background:transparent;color:var(--dsw-alias-label-secondary);font:inherit;font-size:12px;cursor:pointer}.dcu-goalCreate:hover{background:var(--dsw-alias-interactive-bg-hover);color:var(--dsw-alias-label-primary)}.dcu-goalEditor{display:grid;grid-template-columns:minmax(0,1fr) 120px auto;align-items:center;gap:6px;width:100%;padding:5px 0}.dcu-goalEditor input{box-sizing:border-box;min-width:0;height:30px;padding:0 9px;border:1px solid var(--dsw-alias-border-l2);border-radius:8px;background:var(--dsw-alias-bg-layer-1,#fff);color:inherit;font:inherit;font-size:12px}.dcu-goalError{padding:0 max(16px,calc((100% - 780px)/2)) 5px;color:var(--dsw-alias-state-error-primary,#c33);font-size:11px}@media(max-width:700px){.dcu-goalEditor{grid-template-columns:minmax(0,1fr) auto}.dcu-goalEditor input[type=number]{grid-column:1/-1}.dcu-goalMeta{display:none}}
    `;
    if (!document.querySelector('style[data-plugin-css="dsh-codex-usage"]')) {
      const style = document.createElement("style");
      style.dataset.pluginCss = "dsh-codex-usage";
      style.textContent = CSS;
      document.head.appendChild(style);
    }

    const empty = {
      backend: "dsh", phase: "idle", cliVersion: null, requiredCliVersion: "0.147.0",
      appServerRunning: false, authMode: null, managedInstall: false, state: "unavailable",
      planType: null, email: null, updatedAt: 0, buckets: [], credits: null,
      resetCredits: null, accountUsage: null, threadUsage: {}, error: null,
    };
    const store = {
      value: empty,
      listeners: new Set(),
      source: null,
      getSnapshot() { return this.value; },
      subscribe(listener) { this.listeners.add(listener); return () => this.listeners.delete(listener); },
      set(value) { this.value = value; for (const listener of this.listeners) listener(); },
      async refresh() {
        try {
          const response = await fetch("/desktop-api/codex/usage", { cache: "no-store" });
          if (!response.ok) throw new Error(`HTTP ${response.status}`);
          this.set(await response.json());
        } catch (error) {
          this.set({ ...this.value, state: "unavailable", error: String(error) });
        }
      },
      start() {
        void this.refresh();
        this.source = new EventSource("/desktop-api/codex/usage/events");
        this.source.onmessage = (event) => {
          try { this.set(JSON.parse(event.data)); } catch { /* keep last valid snapshot */ }
        };
      },
      stop() { this.source?.close(); this.source = null; },
    };

    const zh = typeof navigator !== "undefined" && navigator.language.toLowerCase().startsWith("zh");
    const text = (cn, en) => zh ? cn : en;
    const number = (value) => new Intl.NumberFormat().format(value ?? 0);
    const dateTime = (seconds) => seconds ? new Date(seconds * 1000).toLocaleString() : text("未知", "Unknown");
    const updated = (millis) => millis ? new Date(millis).toLocaleString() : text("尚未更新", "Not updated");
    const plan = (value) => value ? value.toUpperCase() : text("未识别", "Unknown");
    const useUsage = () => React.useSyncExternalStore(
      (listener) => store.subscribe(listener),
      () => store.getSnapshot(),
      () => empty,
    );
    const invokeDesktop = (command, args = {}) => {
      const invoke = window.__TAURI_INTERNALS__?.invoke;
      if (typeof invoke !== "function") {
        return Promise.reject(new Error(text("Codex 会话交接只能在 DSH Desktop 中使用。", "Codex handoff is only available inside DSH Desktop.")));
      }
      return invoke(command, args);
    };
    const codexSessionTitle = (prompt, attachments = []) => {
      const firstLine = String(prompt || "").split(/\r?\n/, 1)[0].trim();
      const subject = firstLine || attachments[0]?.name || text("附件任务", "Attachment task");
      return `Codex · ${subject.slice(0, 48)}`;
    };
    const MAX_ATTACHMENTS = 10;
    const MAX_ATTACHMENT_BYTES = 20 * 1024 * 1024;
    const MAX_ATTACHMENTS_BYTES = 50 * 1024 * 1024;
    const fileBase64 = async (file) => {
      const bytes = new Uint8Array(await file.arrayBuffer());
      let binary = "";
      for (let offset = 0; offset < bytes.length; offset += 0x8000) {
        binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
      }
      return btoa(binary);
    };
    function useAttachments() {
      const [attachments, setAttachments] = React.useState([]);
      const [error, setError] = React.useState(null);
      const [reading, setReading] = React.useState(false);
      const inputRef = React.useRef(null);
      const choose = async (event) => {
        const selected = Array.from(event.target.files || []);
        event.target.value = "";
        if (!selected.length) return;
        if (attachments.length + selected.length > MAX_ATTACHMENTS) {
          setError(text(`每条消息最多添加 ${MAX_ATTACHMENTS} 个文件。`, `Up to ${MAX_ATTACHMENTS} files can be attached to one message.`));
          return;
        }
        const total = attachments.reduce((sum, item) => sum + item.size, 0) + selected.reduce((sum, file) => sum + file.size, 0);
        const oversized = selected.find((file) => file.size > MAX_ATTACHMENT_BYTES);
        if (oversized) {
          setError(text(`“${oversized.name}”超过 20 MB。`, `“${oversized.name}” is larger than 20 MB.`));
          return;
        }
        if (total > MAX_ATTACHMENTS_BYTES) {
          setError(text("单条消息的附件总大小不能超过 50 MB。", "Attachments in one message cannot exceed 50 MB in total."));
          return;
        }
        setReading(true);
        setError(null);
        try {
          const encoded = [];
          for (const file of selected) {
            encoded.push({
              id: `${file.name}:${file.size}:${file.lastModified}:${Math.random()}`,
              name: file.name,
              size: file.size,
              mimeType: file.type || "application/octet-stream",
              dataBase64: await fileBase64(file),
            });
          }
          setAttachments((current) => [...current, ...encoded]);
        } catch (value) {
          setError(text(`读取文件失败：${String(value)}`, `Could not read the file: ${String(value)}`));
        } finally {
          setReading(false);
        }
      };
      const remove = (id) => setAttachments((current) => current.filter((item) => item.id !== id));
      const clear = () => { setAttachments([]); setError(null); };
      const payload = attachments.map(({ name, mimeType, dataBase64 }) => ({ name, mimeType, dataBase64 }));
      return { attachments, error, reading, inputRef, choose, remove, clear, payload };
    }
    function AttachmentPicker({ uploads, disabled = false }) {
      return h(React.Fragment, null,
        h("input", { ref: uploads.inputRef, className: "dcu-fileInput", type: "file", multiple: true, onChange: uploads.choose, tabIndex: -1 }),
        h("button", { type: "button", className: "dcu-add", disabled: disabled || uploads.reading, onClick: () => uploads.inputRef.current?.click(), "aria-label": text("添加文件", "Add files"), title: text("添加文件", "Add files") }, uploads.reading ? "…" : "+"),
      );
    }
    function AttachmentList({ uploads }) {
      if (!uploads.attachments.length) return null;
      return h("div", { className: "dcu-attachments" }, ...uploads.attachments.map((file) => h("div", { key: file.id, className: "dcu-attachment", title: file.name }, h("span", null, file.name), h("button", { type: "button", onClick: () => uploads.remove(file.id), "aria-label": text(`移除 ${file.name}`, `Remove ${file.name}`) }, "×"))));
    }
    const blockText = (blocks) => (blocks || []).map((block) => {
      if (typeof block?.text === "string") return block.text;
      if (typeof block?.content === "string") return block.content;
      return "";
    }).filter(Boolean).join("\n");
    const nodeText = (node) => {
      if (node?.kind === "assistant") return (node.blocks || []).filter((block) => block.kind === "text").map((block) => block.text).join("\n");
      if (["user", "steering", "context"].includes(node?.kind)) return blockText(node.content);
      if (node?.kind === "turn-error") return `[turn error] ${node.message || ""}`;
      return "";
    };
    const transcript = (nodes) => {
      const rows = [];
      let size = 0;
      for (const node of (nodes || []).slice(-100).reverse()) {
        const body = nodeText(node).trim();
        if (!body) continue;
        const role = node.kind === "assistant" ? "ASSISTANT" : node.kind === "context" ? "CONTEXT" : "USER";
        const row = `${role}: ${body}`;
        if (size + row.length > 120000) break;
        rows.push(row);
        size += row.length;
      }
      return rows.reverse().join("\n\n");
    };
    const approvalLabel = (approval) => {
      const command = approval.command;
      if (Array.isArray(command)) return command.join(" ");
      if (typeof command === "string") return command;
      return approval.grantRoot || approval.reason || text("文件修改请求", "File change request");
    };
    const activityText = (event) => {
      const item = event.params?.item;
      if (event.method === "turn/plan/updated") {
        const items = event.params?.plan || [];
        return items.map((entry) => `${entry.status === "completed" ? "✓" : "•"} ${entry.step}`).join("\n");
      }
      if (event.method === "turn/diff/updated") return text("Codex 更新了工作区文件差异", "Codex updated the workspace diff");
      if (event.method === "warning" || event.method === "error") return event.params?.message || event.params?.error?.message || event.method;
      if (event.method === "item/started" && item?.type === "commandExecution") return `${text("执行", "Running")}: ${Array.isArray(item.command) ? item.command.join(" ") : item.command || "command"}`;
      if (event.method === "item/completed" && item?.type === "commandExecution") return `${text("命令", "Command")}: ${item.status || "completed"}${item.exitCode == null ? "" : ` · exit ${item.exitCode}`}`;
      if (event.method === "item/completed" && item?.type === "fileChange") return text("文件修改已完成", "File changes completed");
      if (event.method === "turn/completed") return event.params?.turn?.status === "failed" ? (event.params?.turn?.error?.message || text("Codex 处理失败", "Codex turn failed")) : "";
      return "";
    };

    function useCodexModels(enabled = true) {
      const [catalog, setCatalog] = React.useState(null);
      const [model, setModel] = React.useState("");
      const [effort, setEffort] = React.useState("");
      const [error, setError] = React.useState(null);
      const [loading, setLoading] = React.useState(false);
      const refresh = React.useCallback(async () => {
        if (!enabled) return;
        setLoading(true);
        try {
          const next = await invokeDesktop("codex_model_catalog");
          setCatalog(next);
          setModel(next.selectedModel || next.data?.[0]?.model || "");
          setEffort(next.selectedEffort || "");
          setError(null);
        } catch (value) {
          setError(String(value));
        } finally {
          setLoading(false);
        }
      }, [enabled]);
      React.useEffect(() => { void refresh(); }, [refresh]);
      const selected = catalog?.data?.find((entry) => entry.model === model) || null;
      const efforts = selected?.supportedReasoningEfforts || [];
      const chooseModel = (value) => {
        const next = catalog?.data?.find((entry) => entry.model === value);
        setModel(value);
        setEffort(next?.defaultReasoningEffort || next?.supportedReasoningEfforts?.[0]?.reasoningEffort || "");
      };
      return { catalog, model, effort, efforts, error, loading, setEffort, chooseModel, refresh };
    }

    function ModelControls({ models, disabled = false }) {
      return h("div", { className: "dcu-models" },
        h("select", {
          className: "dcu-select", value: models.model, disabled: disabled || models.loading || !models.catalog,
          "aria-label": text("Codex 模型", "Codex model"), title: text("Codex 模型", "Codex model"),
          onChange: (event) => models.chooseModel(event.target.value),
        }, ...(models.catalog?.data || []).map((entry) => h("option", { key: entry.model, value: entry.model }, entry.displayName || entry.model))),
        h("select", {
          className: "dcu-select", value: models.effort, disabled: disabled || models.loading || !models.efforts.length,
          "aria-label": text("推理等级", "Reasoning effort"), title: text("推理等级", "Reasoning effort"),
          onChange: (event) => models.setEffort(event.target.value),
        }, ...models.efforts.map((entry) => h("option", { key: entry.reasoningEffort, value: entry.reasoningEffort, title: entry.description }, entry.reasoningEffort))),
      );
    }

    const fallbackModes = [
      { mode: "default", name: "Default" },
      { mode: "plan", name: "Plan" },
    ];
    const modeLabel = (mode) => mode === "plan" ? text("计划模式", "Plan mode") : text("标准模式", "Default mode");
    const modeDescription = (mode) => mode === "plan"
      ? text("先研究并给出可评审计划，不直接实施修改", "Research first and return a reviewable plan without implementing changes")
      : text("执行、修改并验证当前任务", "Execute, modify, and verify the current task");
    function useCodexModes(enabled = true) {
      const [data, setData] = React.useState(fallbackModes);
      const [error, setError] = React.useState(null);
      React.useEffect(() => {
        if (!enabled) return undefined;
        let active = true;
        void invokeDesktop("codex_collaboration_mode_catalog").then((catalog) => {
          if (!active) return;
          const modes = (catalog?.data || []).filter((entry) => ["default", "plan"].includes(entry.mode));
          if (modes.length) setData(modes);
          setError(null);
        }).catch((value) => { if (active) setError(String(value)); });
        return () => { active = false; };
      }, [enabled]);
      return { data, error };
    }
    function CodexModeControl({ value, onChange, modes, disabled = false }) {
      return h("span", { className: "dcu-modeSeat", title: modeDescription(value) },
        h("select", {
          value, disabled, "aria-label": text("Codex 模式", "Codex mode"),
          onChange: (event) => onChange(event.target.value),
        }, ...(modes?.data || fallbackModes).map((entry) => h("option", { key: entry.mode, value: entry.mode }, modeLabel(entry.mode)))),
      );
    }

    const goalStatusLabel = (status) => ({
      active: text("进行中的目标", "Ongoing Goal"),
      paused: text("已暂停的目标", "Paused Goal"),
      blocked: text("受阻的目标", "Blocked Goal"),
      usageLimited: text("用量受限", "Usage limited"),
      budgetLimited: text("预算已用尽", "Budget reached"),
      complete: text("目标已完成", "Goal complete"),
    })[status] || status;
    function CodexGoalBar({ sessionId, linked, goal, running, onChanged }) {
      const [editing, setEditing] = React.useState(false);
      const [objective, setObjective] = React.useState("");
      const [tokenBudget, setTokenBudget] = React.useState("");
      const [busy, setBusy] = React.useState(false);
      const [error, setError] = React.useState(null);
      React.useEffect(() => {
        setEditing(false);
        setError(null);
      }, [sessionId]);
      const beginEdit = () => {
        setObjective(goal?.objective || "");
        setTokenBudget(goal?.tokenBudget == null ? "" : String(goal.tokenBudget));
        setEditing(true);
        setError(null);
      };
      const update = async (patch) => {
        if (busy) return;
        setBusy(true);
        setError(null);
        try {
          const next = await invokeDesktop("codex_session_goal_update", { request: { sessionId, ...patch } });
          onChanged(next);
          return next;
        } catch (value) {
          setError(String(value));
          return null;
        } finally {
          setBusy(false);
        }
      };
      const save = async () => {
        const value = objective.trim();
        if (!value) return;
        const patch = { objective: value };
        if (tokenBudget.trim()) patch.tokenBudget = Number(tokenBudget);
        if (await update(patch)) setEditing(false);
      };
      const clear = async () => {
        if (busy) return;
        setBusy(true);
        setError(null);
        try {
          await invokeDesktop("codex_session_goal_clear", { sessionId });
          onChanged(null);
        } catch (value) { setError(String(value)); }
        finally { setBusy(false); }
      };
      if (!linked) return null;
      if (editing) return h(React.Fragment, null,
        h("div", { className: "dcu-goalDock" }, h("div", { className: "dcu-goalBar" },
          h("span", { className: "dcu-goalGlyph" }, "◎"),
          h("div", { className: "dcu-goalEditor" },
            h("input", { value: objective, maxLength: 4000, autoFocus: true, placeholder: text("目标内容", "Goal objective"), onChange: (event) => setObjective(event.target.value), onKeyDown: (event) => { if (event.key === "Enter") void save(); if (event.key === "Escape") setEditing(false); } }),
            h("input", { type: "number", min: 1, value: tokenBudget, placeholder: text("Token 预算（可选）", "Token budget (optional)"), onChange: (event) => setTokenBudget(event.target.value) }),
            h("div", { className: "dcu-goalActions" },
              h("button", { type: "button", className: "dcu-goalIcon", disabled: busy || !objective.trim(), onClick: () => void save(), "aria-label": text("保存目标", "Save goal"), title: text("保存目标", "Save goal") }, "✓"),
              h("button", { type: "button", className: "dcu-goalIcon", disabled: busy, onClick: () => setEditing(false), "aria-label": text("取消编辑", "Cancel edit"), title: text("取消编辑", "Cancel edit") }, "×"),
            ),
          ),
        )),
        error ? h("div", { className: "dcu-goalError", role: "alert" }, error) : null,
      );
      if (!goal) return h(React.Fragment, null,
        h("div", { className: "dcu-goalDock" }, h("button", { type: "button", className: "dcu-goalCreate", disabled: running, onClick: beginEdit }, `◎ ${text("设定目标", "Set goal")}`)),
        error ? h("div", { className: "dcu-goalError", role: "alert" }, error) : null,
      );
      const budget = goal.tokenBudget ? `${number(goal.tokensUsed)} / ${number(goal.tokenBudget)}` : `${number(goal.tokensUsed)} tokens`;
      return h(React.Fragment, null,
        h("div", { className: "dcu-goalDock" }, h("div", { className: "dcu-goalBar", title: goal.objective },
          h("span", { className: "dcu-goalGlyph" }, "◎"),
          h("span", { className: "dcu-goalLabel" }, goalStatusLabel(goal.status)),
          h("span", { className: "dcu-goalObjective" }, goal.objective),
          h("span", { className: "dcu-goalMeta" }, budget),
          h("div", { className: "dcu-goalActions" },
            goal.status === "active" ? h("button", { type: "button", className: "dcu-goalIcon", disabled: busy || running, onClick: () => void update({ status: "paused" }), "aria-label": text("暂停目标", "Pause goal"), title: text("暂停目标", "Pause goal") }, "Ⅱ") : null,
            goal.status === "paused" ? h("button", { type: "button", className: "dcu-goalIcon", disabled: busy || running, onClick: () => void update({ status: "active" }), "aria-label": text("恢复目标", "Resume goal"), title: text("恢复目标", "Resume goal") }, "▶") : null,
            h("button", { type: "button", className: "dcu-goalIcon", disabled: busy || running, onClick: beginEdit, "aria-label": text("编辑目标", "Edit goal"), title: text("编辑目标", "Edit goal") }, "✎"),
            h("button", { type: "button", className: "dcu-goalIcon", disabled: busy || running, onClick: () => void clear(), "aria-label": text("清除目标", "Clear goal"), title: text("清除目标", "Clear goal") }, "⌫"),
          ),
        )),
        error ? h("div", { className: "dcu-goalError", role: "alert" }, error) : null,
      );
    }

    function CodexSessionDrawer({ sessionId, useSession, useSessions, useInput, inputActions, onClose, embedded = false, startCodexSession, markCodexSession, collaborationMode = "default", onCollaborationModeChange, onSessionMode, modes }) {
      const usage = useUsage();
      const nodes = useSession((state) => state.nodes);
      const draft = useInput((state) => state.draft);
      const summary = useSessions((state) => state.byId[sessionId]);
      const [snapshot, setSnapshot] = React.useState(null);
      const [prompt, setPrompt] = React.useState("");
      const [error, setError] = React.useState(null);
      const [sending, setSending] = React.useState(false);
      const [liveText, setLiveText] = React.useState("");
      const [livePlan, setLivePlan] = React.useState("");
      const [activities, setActivities] = React.useState([]);
      const [approvals, setApprovals] = React.useState([]);
      const uploads = useAttachments();
      const cursor = React.useRef(0);
      const polling = React.useRef(false);
      const initial = React.useRef(true);
      const goalFetched = React.useRef(false);
      const chat = React.useRef(null);
      const models = useCodexModels(usage.backend === "codex" && usage.appServerRunning);

      React.useEffect(() => {
        setPrompt(draft || "");
        cursor.current = 0;
        initial.current = true;
        goalFetched.current = false;
        setLiveText("");
        setLivePlan("");
      }, [sessionId]);

      const applyEvents = React.useCallback((events, first, activeTurnId) => {
        const nextActivities = [];
        for (const event of events || []) {
          if (event.method.endsWith("/requestApproval")) {
            setApprovals((values) => values.some((item) => item.requestId === event.params?.requestId) ? values : [...values, event.params]);
            continue;
          }
          if (event.method === "serverRequest/resolved") {
            const requestId = event.params?.requestId;
            setApprovals((values) => values.filter((item) => item.requestId !== requestId));
            continue;
          }
          if ((!first || (activeTurnId && event.params?.turnId === activeTurnId)) && event.method === "item/agentMessage/delta") setLiveText((value) => value + (event.params?.delta || ""));
          if ((!first || (activeTurnId && event.params?.turnId === activeTurnId)) && event.method === "item/plan/delta") setLivePlan((value) => value + (event.params?.delta || ""));
          if (event.method === "item/completed" && event.params?.item?.type === "agentMessage") setLiveText("");
          if (event.method === "item/completed" && event.params?.item?.type === "plan") setLivePlan("");
          if (event.method === "turn/completed") { setLiveText(""); setLivePlan(""); }
          const label = activityText(event);
          if (label) nextActivities.push({ seq: event.seq, text: label });
        }
        if (nextActivities.length) setActivities((values) => [...values, ...nextActivities].slice(-20));
      }, []);

      const poll = React.useCallback(async () => {
        if (polling.current) return;
        polling.current = true;
        try {
          const next = await invokeDesktop("codex_session_poll", { sessionId, afterSeq: cursor.current });
          const first = initial.current;
          initial.current = false;
          applyEvents(next.events, first, next.activeTurnId);
          cursor.current = next.latestSeq || cursor.current;
          setSnapshot(next);
          onSessionMode?.(next.collaborationMode || "default");
          if (next.linked && !goalFetched.current) {
            goalFetched.current = true;
            void invokeDesktop("codex_session_goal_get", { sessionId }).then((goal) => {
              setSnapshot((current) => current ? { ...current, goal } : current);
            }).catch((value) => setError(String(value)));
          }
          setError(null);
        } catch (value) {
          setError(String(value));
        } finally {
          polling.current = false;
        }
      }, [sessionId, applyEvents, onSessionMode]);

      React.useEffect(() => {
        void poll();
        const timer = setInterval(() => void poll(), 450);
        return () => clearInterval(timer);
      }, [poll]);
      React.useEffect(() => {
        if (chat.current) chat.current.scrollTop = chat.current.scrollHeight;
      }, [snapshot?.messages?.length, liveText, livePlan, activities.length, approvals.length]);
      React.useEffect(() => {
        if (embedded) return undefined;
        const escape = (event) => { if (event.key === "Escape") onClose(); };
        document.addEventListener("keydown", escape);
        return () => document.removeEventListener("keydown", escape);
      }, [embedded, onClose]);

      const send = async () => {
        const value = prompt.trim();
        if ((!value && !uploads.attachments.length) || sending) return;
        if (!summary?.cwd) {
          setError(text("当前 dsh 会话没有可用的工作区目录。", "The current dsh session has no workspace directory."));
          return;
        }
        setSending(true);
        setError(null);
        try {
          const firstTitle = snapshot?.linked ? (snapshot.title || summary.title || summary.displayTitle || null) : codexSessionTitle(value, uploads.attachments);
          await invokeDesktop("codex_session_send", { request: {
            sessionId,
            cwd: summary.cwd,
            prompt: value || text("请查看并处理附件。", "Review and process the attached files."),
            context: snapshot?.linked ? null : transcript(nodes),
            title: firstTitle,
            model: models.model || null,
            effort: models.effort || null,
            collaborationMode,
            attachments: uploads.payload,
          } });
          if (!snapshot?.linked) await markCodexSession?.(sessionId, firstTitle);
          if ((draft || "").trim() === value) inputActions.setDraft("");
          setPrompt("");
          uploads.clear();
          await poll();
        } catch (value) {
          setError(String(value));
        } finally {
          setSending(false);
        }
      };
      const stop = async () => {
        try { await invokeDesktop("codex_session_interrupt", { sessionId }); await poll(); }
        catch (value) { setError(String(value)); }
      };
      const reset = async () => {
        try {
          if (typeof startCodexSession !== "function") throw new Error(text("当前 dsh 版本不支持创建独立 Codex 会话。", "This dsh version cannot create an independent Codex session."));
          await startCodexSession({ cwd: summary?.cwd });
          if (!embedded) onClose?.();
        } catch (value) { setError(String(value)); }
      };
      const approve = async (requestId, decision) => {
        try {
          await invokeDesktop("codex_session_approve", { sessionId, requestId, decision });
          setApprovals((values) => values.filter((item) => item.requestId !== requestId));
        } catch (value) { setError(String(value)); }
      };
      const running = Boolean(snapshot?.activeTurnId);
      const ready = usage.backend === "codex" && usage.appServerRunning && usage.authMode === "chatgpt" && usage.phase === "ready";
      return h(React.Fragment, null,
        embedded ? null : h("div", { className: "dcu-drawerBackdrop", onMouseDown: onClose, "aria-hidden": true }),
        h(embedded ? "section" : "aside", { className: embedded ? "dcu-sessionView" : "dcu-drawer", role: embedded ? "region" : "dialog", "aria-modal": embedded ? undefined : true, "aria-label": text("Codex 会话", "Codex session") },
          h("div", { className: "dcu-drawerHead" },
            h("div", { className: "dcu-drawerTitle" }, h("strong", null, running ? text("Codex 正在处理", "Codex is working") : text("Codex 会话", "Codex session")), h("small", { title: summary?.cwd }, summary?.cwd || sessionId)),
            h("div", { className: "dcu-drawerActions" }, running ? h("button", { className: "dcu-danger", onClick: stop }, text("停止", "Stop")) : null, snapshot?.linked ? h("button", { className: "dcu-secondary", disabled: running, onClick: reset }, text("新建 Codex 会话", "New Codex session")) : null, embedded ? null : h("button", { className: "dcu-iconBtn", onClick: onClose, "aria-label": text("关闭", "Close") }, "×")),
          ),
          !ready ? h("div", { className: "dcu-error", style: { margin: 16 } }, text("Codex 尚未就绪。请在 dsh 的「设置 → Codex」中完成检测、安装和 ChatGPT 登录。", "Codex is not ready. Complete detection, installation, and ChatGPT login in dsh Settings → Codex.")) : null,
          error ? h("div", { className: "dcu-error", style: { margin: "12px 16px 0" } }, error) : null,
          h("div", { className: "dcu-chat", ref: chat },
            !(snapshot?.messages?.length) && !liveText ? h("div", { className: "dcu-empty" }, h("strong", null, text("把当前工作上下文交给 Codex", "Hand the current work context to Codex")), h("p", null, text("首次发送会附带当前 dsh 对话摘要；后续消息会继续同一个本地 Codex thread。", "The first send includes the current dsh transcript; later messages continue the same local Codex thread."))) : null,
            ...(snapshot?.messages || []).map((message) => h("div", { key: message.id, className: `dcu-msg ${message.role}` }, message.role === "plan" ? h(React.Fragment, null, h("span", { className: "dcu-planLabel" }, text("计划", "Plan")), message.text) : message.text)),
            liveText ? h("div", { className: "dcu-msg assistant live" }, liveText) : null,
            livePlan ? h("div", { className: "dcu-msg plan live" }, h("span", { className: "dcu-planLabel" }, text("计划", "Plan")), livePlan) : null,
            ...activities.map((item) => h("div", { key: item.seq, className: "dcu-activity" }, item.text)),
            ...approvals.map((approval) => h("div", { key: approval.requestId, className: "dcu-approval" },
              h("strong", null, approval.kind === "command" ? text("Codex 请求执行命令", "Codex requests command execution") : text("Codex 请求修改文件", "Codex requests file changes")),
              approval.reason ? h("p", null, approval.reason) : null,
              h("pre", null, approvalLabel(approval)),
              h("div", { className: "dcu-approvalActions" }, h("button", { className: "dcu-primary", onClick: () => approve(approval.requestId, "accept") }, text("允许一次", "Allow once")), h("button", { className: "dcu-secondary", onClick: () => approve(approval.requestId, "acceptForSession") }, text("本会话允许", "Allow for session")), h("button", { className: "dcu-danger", onClick: () => approve(approval.requestId, "decline") }, text("拒绝", "Decline"))),
            )),
          ),
          h(CodexGoalBar, { sessionId, linked: Boolean(snapshot?.linked), goal: snapshot?.goal || null, running, onChanged: (goal) => setSnapshot((current) => current ? { ...current, goal } : current) }),
          embedded ? null : h("div", { className: "dcu-compose" },
            h("textarea", { value: prompt, disabled: !ready || sending, placeholder: snapshot?.linked ? text("继续告诉 Codex 要做什么…", "Tell Codex what to do next…") : text("确认或补充要交给 Codex 的当前任务…", "Confirm or add to the task being handed to Codex…"), onChange: (event) => setPrompt(event.target.value), onKeyDown: (event) => { if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) { event.preventDefault(); void send(); } } }),
            h(AttachmentList, { uploads }),
            h("div", { className: "dcu-composeFoot" }, h("div", { className: "dcu-composeTools" }, h(AttachmentPicker, { uploads, disabled: !ready || sending || running }), h(CodexModeControl, { value: collaborationMode, onChange: onCollaborationModeChange, modes, disabled: sending || running }), h(ModelControls, { models, disabled: sending || running })), h("div", { className: "dcu-composeButtons" }, running ? h("button", { className: "dcu-danger", onClick: stop }, text("停止", "Stop")) : null, h("button", { className: "dcu-primary", disabled: !ready || sending || (!prompt.trim() && !uploads.attachments.length) || !models.model, onClick: send }, sending ? text("正在发送…", "Sending…") : snapshot?.linked ? text("发送给 Codex", "Send to Codex") : text("交给 Codex", "Hand off")))),
            uploads.error || models.error || modes?.error ? h("div", { className: "dcu-composerError" }, uploads.error || models.error || modes?.error) : null,
          ),
        ),
      );
    }

    function CodexConversationView(props) {
      return h(CodexSessionDrawer, { ...props, embedded: true, onClose: () => {} });
    }

    function CodexDrawerPanel(props) {
      const usage = useUsage();
      const modes = useCodexModes(usage.backend === "codex" && usage.appServerRunning);
      const [collaborationMode, setCollaborationMode] = React.useState("default");
      const modeTouched = React.useRef(false);
      React.useEffect(() => {
        modeTouched.current = false;
        setCollaborationMode("default");
      }, [props.sessionId]);
      const chooseMode = React.useCallback((mode) => {
        modeTouched.current = true;
        setCollaborationMode(mode);
      }, []);
      const receiveSessionMode = React.useCallback((mode) => {
        if (!modeTouched.current) setCollaborationMode(mode || "default");
      }, []);
      return h(CodexSessionDrawer, {
        ...props,
        collaborationMode,
        onCollaborationModeChange: chooseMode,
        onSessionMode: receiveSessionMode,
        modes,
      });
    }

    function CodexComposerCard(props) {
      const usage = useUsage();
      const draft = props.useInput((state) => state.draft);
      const nodes = props.useSession((state) => state.nodes);
      const summary = props.useSessions((state) => state.byId[props.sessionId]);
      const [sending, setSending] = React.useState(false);
      const [error, setError] = React.useState(null);
      const uploads = useAttachments();
      const models = useCodexModels(usage.backend === "codex" && usage.appServerRunning);
      const ready = usage.backend === "codex" && usage.appServerRunning && usage.authMode === "chatgpt" && usage.phase === "ready";
      const send = async () => {
        const value = (draft || "").trim();
        if ((!value && !uploads.attachments.length) || sending || !ready || !models.model) return;
        if (!summary?.cwd) {
          setError(text("当前会话没有可用的工作区目录。", "The current session has no workspace directory."));
          return;
        }
        setSending(true);
        setError(null);
        try {
          const snapshot = await invokeDesktop("codex_session_poll", { sessionId: props.sessionId, afterSeq: 0 });
          const firstTitle = snapshot?.linked ? (snapshot.title || summary.title || summary.displayTitle || null) : codexSessionTitle(value, uploads.attachments);
          await invokeDesktop("codex_session_send", { request: {
            sessionId: props.sessionId,
            cwd: summary.cwd,
            prompt: value || text("请查看并处理附件。", "Review and process the attached files."),
            context: snapshot?.linked ? null : transcript(nodes),
            title: firstTitle,
            model: models.model,
            effort: models.effort || null,
            collaborationMode: props.collaborationMode,
            attachments: uploads.payload,
          } });
          if (!snapshot?.linked) await props.markCodexSession?.(props.sessionId, firstTitle);
          props.inputActions.setDraft("");
          uploads.clear();
        } catch (value) {
          setError(String(value));
        } finally {
          setSending(false);
        }
      };
      return h("div", { className: "dcu-mainComposer", "data-codex-composer": "" },
          h("textarea", {
            value: draft || "", disabled: sending, placeholder: ready ? text("描述你希望 Codex 完成的任务", "Describe a task for Codex") : text("Codex 正在准备中…", "Codex is getting ready…"),
            onChange: (event) => props.inputActions.setDraft(event.target.value),
            onKeyDown: (event) => {
              if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent?.isComposing) {
                event.preventDefault();
                void send();
              }
            },
          }),
          h(AttachmentList, { uploads }),
          h("div", { className: "dcu-mainFoot" },
            h(AttachmentPicker, { uploads, disabled: !ready || sending }),
            h("div", { className: "dcu-mainTools" }, h(ModelControls, { models, disabled: sending }), h("button", { type: "button", className: "dcu-send", disabled: !ready || sending || (!(draft || "").trim() && !uploads.attachments.length) || !models.model, onClick: send, "aria-label": text("发送给 Codex", "Send to Codex"), title: text("发送给 Codex", "Send to Codex") }, sending ? "…" : "↑")),
          ),
          error || uploads.error || models.error || props.modes?.error ? h("div", { className: "dcu-composerError" }, error || uploads.error || models.error || props.modes?.error) : null,
        );
    }

    function CodexComposer(props) {
      const usage = useUsage();
      const modes = useCodexModes(usage.backend === "codex" && usage.appServerRunning);
      const [collaborationMode, setCollaborationMode] = React.useState("default");
      const modeTouched = React.useRef(false);
      React.useEffect(() => {
        modeTouched.current = false;
        setCollaborationMode("default");
      }, [props.sessionId]);
      const chooseMode = React.useCallback((mode) => {
        modeTouched.current = true;
        setCollaborationMode(mode);
      }, []);
      const receiveSessionMode = React.useCallback((mode) => {
        if (!modeTouched.current) setCollaborationMode(mode || "default");
      }, []);
      const controlled = {
        ...props,
        collaborationMode,
        onCollaborationModeChange: chooseMode,
        onSessionMode: receiveSessionMode,
        modes,
      };
      return h("section", { className: "dcu-codexShell", "aria-label": text("Codex 会话", "Codex session") },
        h(CodexConversationView, controlled),
        h("div", { className: "dcu-modeRow" }, h(CodexModeControl, { value: collaborationMode, onChange: chooseMode, modes })),
        h("div", { className: "dcu-codexComposerSeat" }, h(CodexComposerCard, controlled)),
      );
    }

    function CodexHandoffAction(props) {
      const usage = useUsage();
      const [open, setOpen] = React.useState(false);
      const linked = usage.backend === "codex" && usage.appServerRunning && usage.authMode === "chatgpt";
      if (usage.backend !== "codex") return null;
      return h(React.Fragment, null,
        h("button", { type: "button", className: `dcu-handoff ${linked ? "live" : ""}`, onClick: () => setOpen(true), title: text("将当前工作交给本机 Codex", "Hand current work to local Codex") }, text("交给 Codex", "Hand off to Codex")),
        open ? h(CodexDrawerPanel, { ...props, onClose: () => setOpen(false) }) : null,
      );
    }

    const phaseLabels = {
      idle: ["未启用", "Not enabled"],
      detecting: ["正在检测 Codex CLI…", "Detecting Codex CLI…"],
      installing: ["正在下载并安装 Codex CLI…", "Downloading and installing Codex CLI…"],
      startingAppServer: ["正在启动 Codex app-server…", "Starting Codex app-server…"],
      checkingAccount: ["正在检查 ChatGPT 登录状态…", "Checking ChatGPT login…"],
      startingLogin: ["正在创建 ChatGPT 登录…", "Starting ChatGPT login…"],
      waitingForLogin: ["等待浏览器完成 ChatGPT 登录…", "Waiting for ChatGPT login in the browser…"],
      signingOut: ["正在退出 ChatGPT…", "Signing out of ChatGPT…"],
      signedOut: ["已退出 ChatGPT", "Signed out of ChatGPT"],
      ready: ["Codex 已就绪", "Codex is ready"],
      error: ["配置失败", "Configuration failed"],
    };
    const busyPhases = new Set(["detecting", "installing", "startingAppServer", "checkingAccount", "startingLogin", "waitingForLogin", "signingOut"]);
    const phaseLabel = (phase) => {
      const value = phaseLabels[phase];
      return value ? text(value[0], value[1]) : phase;
    };

    function CodexControl() {
      const [status, setStatus] = React.useState(null);
      const [error, setError] = React.useState(null);
      const [acting, setActing] = React.useState(false);
      const mounted = React.useRef(true);
      const lastUsageStatus = React.useRef("");

      const readStatus = React.useCallback(async (detect = false) => {
        try {
          const next = await invokeDesktop(detect ? "codex_status" : "codex_status_cached");
          if (mounted.current) {
            setStatus(next);
            setError(null);
            const usageStatus = `${next.backend}:${next.phase}:${next.appServerRunning}:${next.authMode || ""}`;
            if (usageStatus !== lastUsageStatus.current) {
              lastUsageStatus.current = usageStatus;
              void store.refresh();
            }
          }
          return next;
        } catch (value) {
          if (mounted.current) setError(String(value));
          return null;
        }
      }, []);

      React.useEffect(() => {
        mounted.current = true;
        void readStatus(true);
        const timer = setInterval(() => void readStatus(false), 1000);
        return () => { mounted.current = false; clearInterval(timer); };
      }, [readStatus]);

      const run = async (command, args) => {
        setActing(true);
        setError(null);
        try {
          await invokeDesktop(command, args);
          await readStatus(false);
          setTimeout(() => { void readStatus(false); void store.refresh(); }, 350);
        } catch (value) {
          setError(String(value));
        } finally {
          setActing(false);
        }
      };
      const refresh = async () => {
        setActing(true);
        setError(null);
        try {
          const next = await readStatus(true);
          if (next?.appServerRunning) await invokeDesktop("codex_usage_refresh");
          await store.refresh();
        } catch (value) {
          setError(String(value));
        } finally {
          setActing(false);
        }
      };

      if (!status) return h("div", { className: "dcu-control" }, h("div", { className: "dcu-phase" }, h("span", { className: "dcu-spinner" }), text("正在读取后端状态…", "Loading backend status…")), error ? h("div", { className: "dcu-error" }, error) : null);
      const busy = acting || busyPhases.has(status.phase);
      const codexReady = status.backend === "codex" && status.phase === "ready" && status.appServerRunning && status.authMode === "chatgpt";
      return h("div", { className: "dcu-control" },
        h("div", { className: "dcu-controlHead" },
          h("div", null, h("h3", null, text("会话后端", "Session backend")), h("p", { className: "dcu-sub" }, text("直接在此检测、安装并启用本机 Codex。", "Detect, install, and enable local Codex here."))),
          h("button", { type: "button", className: "dcu-refresh", disabled: busy, onClick: refresh }, text("重新检测", "Recheck")),
        ),
        h("div", { className: "dcu-backends" },
          h("article", { className: `dcu-backend ${status.backend === "dsh" ? "selected" : ""}` },
            h("div", { className: "dcu-backendTitle" }, h("strong", null, "DeepSeek Harness"), h("span", { className: "dcu-version" }, status.dshVersion)),
            h("p", { className: "dcu-sub" }, text("使用内置 dsh agent、会话存储与工具链。", "Use the built-in dsh agent, session storage, and tools.")),
            h("div", { className: "dcu-actions" }, h("button", { className: "dcu-secondary", disabled: busy || status.backend === "dsh", onClick: () => run("backend_select", { backend: "dsh" }) }, status.backend === "dsh" ? text("当前使用", "In use") : text("切换到 dsh", "Use dsh"))),
          ),
          h("article", { className: `dcu-backend ${status.backend === "codex" ? "selected" : ""}` },
            h("div", { className: "dcu-backendTitle" }, h("strong", null, "OpenAI Codex"), h("span", { className: "dcu-version" }, status.requiredVersion)),
            h("p", { className: "dcu-sub" }, text("缺失或版本不匹配时自动安装，然后启动本地 app-server。", "Automatically install the pinned CLI when needed, then start the local app-server.")),
            h("div", { className: "dcu-statusGrid" },
              h("div", null, h("small", null, "CLI"), h("span", { className: status.exactVersion ? "dcu-good" : "" }, status.installed ? `${status.version}${status.exactVersion ? text(" · 已匹配", " · matched") : text(" · 需升级", " · update needed")}` : text("未检测到", "Not detected"))),
              h("div", null, h("small", null, "app-server"), h("span", { className: status.appServerRunning ? "dcu-good" : "" }, status.appServerRunning ? text("运行中", "Running") : text("未运行", "Stopped"))),
              h("div", null, h("small", null, text("认证", "Authentication")), h("span", { className: status.authMode === "chatgpt" ? "dcu-good" : "" }, status.authMode === "chatgpt" ? `ChatGPT${status.planType ? ` · ${status.planType}` : ""}` : (status.authMode || text("未登录", "Not signed in")))),
              h("div", null, h("small", null, text("安装来源", "Install source")), h("span", null, status.managedInstall ? text("客户端托管", "App managed") : status.installed ? text("系统 CLI", "System CLI") : "—")),
            ),
            status.accountEmail ? h("div", { className: "dcu-row" }, h("span", null, text("账户", "Account")), h("span", { title: status.accountEmail }, status.accountEmail)) : null,
            h("div", { className: "dcu-actions" },
              h("button", { className: "dcu-primary", disabled: busy, onClick: () => run("backend_select", { backend: "codex" }) }, codexReady ? text("重新检测并修复", "Recheck and repair") : text("配置并启用 Codex", "Configure and enable Codex")),
              status.appServerRunning && status.authMode !== "chatgpt" ? h("button", { className: "dcu-secondary", disabled: busy, onClick: () => run("codex_login_chatgpt") }, text("使用 ChatGPT 登录", "Sign in with ChatGPT")) : null,
              status.appServerRunning && status.authMode === "chatgpt" ? h("button", { className: "dcu-secondary", disabled: busy, onClick: () => run("codex_logout_chatgpt") }, text("退出 ChatGPT", "Sign out of ChatGPT")) : null,
            ),
          ),
        ),
        h("div", { className: `dcu-phase ${status.phase === "error" ? "dcu-error" : ""}` }, busy ? h("span", { className: "dcu-spinner" }) : null, h("span", null, phaseLabel(status.phase))),
        status.error ? h("div", { className: "dcu-error" }, status.error) : null,
        error ? h("div", { className: "dcu-error" }, error) : null,
      );
    }

    function Window({ label, value }) {
      if (!value) return null;
      const remaining = Math.max(0, Math.min(100, value.remainingPercent ?? 0));
      return h("div", { className: "dcu-window" },
        h("div", { className: "dcu-row" }, h("span", null, label), h("strong", null, `${remaining}%`)),
        h("div", { className: `dcu-bar ${remaining < 20 ? "warn" : ""}` }, h("i", { style: { width: `${remaining}%` } })),
        h("div", { className: "dcu-row dcu-muted" },
          h("span", null, text("已用", "Used"), ` ${value.usedPercent}%`),
          h("span", null, text("重置于 ", "Resets "), dateTime(value.resetsAt)),
        ),
      );
    }

    function Bucket({ bucket }) {
      return h("article", { className: "dcu-card" },
        h("h3", null, bucket.name || bucket.id || "Codex"),
        h(Window, { label: text("主额度剩余", "Primary remaining"), value: bucket.primary }),
        h(Window, { label: text("次额度剩余", "Secondary remaining"), value: bucket.secondary }),
        bucket.reachedType ? h("div", { className: "dcu-error" }, bucket.reachedType) : null,
      );
    }

    function RuntimeCard({ usage }) {
      return h("article", { className: "dcu-card" }, h("h3", null, text("Codex 运行状态", "Codex runtime")),
        h("div", { className: "dcu-row" }, h("span", null, text("后端", "Backend")), h("span", { className: "dcu-pill" }, usage.backend)),
        h("div", { className: "dcu-row" }, h("span", null, "CLI"), h("span", null, usage.cliVersion || text("未检测", "Not detected"))),
        h("div", { className: "dcu-row" }, h("span", null, text("要求版本", "Required")), h("span", null, usage.requiredCliVersion)),
        h("div", { className: "dcu-row" }, h("span", null, "app-server"), h("span", null, usage.appServerRunning ? text("运行中", "Running") : text("未运行", "Stopped"))),
        h("div", { className: "dcu-row" }, h("span", null, text("安装来源", "Install")), h("span", null, usage.managedInstall ? text("客户端托管", "App managed") : text("系统", "System"))),
      );
    }

    function AccountCard({ usage }) {
      return h("article", { className: "dcu-card" }, h("h3", null, text("账户与订阅", "Account & plan")),
        h("div", { className: "dcu-row" }, h("span", null, text("账户", "Account")), h("span", null, usage.email || "—")),
        h("div", { className: "dcu-row" }, h("span", null, text("套餐", "Plan")), h("strong", null, plan(usage.planType))),
        h("div", { className: "dcu-row" }, h("span", null, text("认证", "Auth")), h("span", null, usage.authMode === "chatgpt" ? "ChatGPT OAuth" : (usage.authMode || "—"))),
        h("div", { className: "dcu-row" }, h("span", null, text("阶段", "Phase")), h("span", null, usage.phase)),
      );
    }

    function CreditsCard({ usage }) {
      if (!usage.credits && !usage.resetCredits) return null;
      return h("article", { className: "dcu-card" }, h("h3", null, "Credits"),
        usage.credits ? h(React.Fragment, null,
          h("div", { className: "dcu-row" }, h("span", null, text("余额", "Balance")), h("strong", null, usage.credits.unlimited ? "∞" : (usage.credits.balance ?? "—"))),
          h("div", { className: "dcu-row" }, h("span", null, text("可用", "Available")), h("span", null, usage.credits.hasCredits ? text("是", "Yes") : text("否", "No"))),
        ) : null,
        usage.resetCredits ? h("div", { className: "dcu-row" }, h("span", null, text("重置次数", "Resets")), h("strong", null, usage.resetCredits.availableCount)) : null,
      );
    }

    function HistoryCard({ account }) {
      if (!account) return null;
      // Bound the number of flex items as well as their minimum size. Account
      // history can span hundreds of days; rendering every bucket used to make
      // their gaps alone wider than the card and stretch the whole settings page.
      const daily = (account.dailyBuckets || []).slice(-120);
      const max = Math.max(1, ...daily.map((item) => item.tokens));
      return h("article", { className: "dcu-card" }, h("h3", null, text("使用统计", "Usage statistics")),
        h("div", { className: "dcu-row" }, h("span", null, text("累计 Tokens", "Lifetime tokens")), h("strong", null, number(account.lifetimeTokens))),
        h("div", { className: "dcu-row" }, h("span", null, text("单日峰值", "Peak day")), h("span", null, number(account.peakDailyTokens))),
        h("div", { className: "dcu-row" }, h("span", null, text("连续使用", "Current streak")), h("span", null, `${account.currentStreakDays ?? 0}d`)),
        daily.length ? h("div", { className: "dcu-history", title: text("每日 Token 使用", "Daily token usage") }, daily.map((item) => h("i", { key: item.startDate, title: `${item.startDate}: ${number(item.tokens)}`, style: { height: `${Math.max(4, item.tokens / max * 100)}%` } }))) : null,
      );
    }

    function ThreadsCard({ threads }) {
      const rows = Object.entries(threads || {}).sort((a, b) => b[1].updatedAt - a[1].updatedAt).slice(0, 8);
      if (!rows.length) return null;
      return h("article", { className: "dcu-card" }, h("h3", null, text("最近 Codex 线程", "Recent Codex threads")),
        h("div", { className: "dcu-threads" }, rows.map(([id, item]) => h("div", { className: "dcu-thread", key: id },
          h("div", { className: "dcu-row" }, h("span", { title: id }, `${id.slice(0, 10)}…`), h("strong", null, number(item.total.totalTokens), " tokens")),
          h("div", { className: "dcu-row dcu-muted" }, h("span", null, text("最近一轮", "Last turn"), ` ${number(item.last.totalTokens)}`), h("span", null, item.modelContextWindow ? `${Math.round(item.total.totalTokens / item.modelContextWindow * 100)}% context` : "")),
        ))),
      );
    }

    function Compact({ usage }) {
      const primary = usage.buckets?.find((item) => item.primary)?.primary;
      const remaining = primary?.remainingPercent ?? 0;
      return h("div", null,
        h(AccountCard, { usage }),
        primary ? h("div", { style: { marginTop: 10 } }, h(Window, { label: text("主额度剩余", "Primary remaining"), value: primary })) : null,
        h("div", { className: "dcu-row dcu-muted", style: { marginTop: 8 } }, h("span", null, text("更新时间", "Updated")), h("span", null, updated(usage.updatedAt))),
      );
    }

    function SidebarUsage({ wide }) {
      const usage = useUsage();
      const [open, setOpen] = React.useState(false);
      const primary = usage.buckets?.find((item) => item.primary)?.primary;
      const remaining = primary?.remainingPercent ?? 0;
      const live = usage.backend === "codex" && usage.appServerRunning;
      return h(React.Fragment, null,
        h("button", { type: "button", className: "dcu-action", onClick: () => setOpen(!open), title: text("Codex 用量", "Codex usage"), "aria-expanded": open },
          primary ? h("span", { className: "dcu-ring", style: { "--p": `${remaining}%` } }) : h("span", { className: `dcu-dot ${live ? "" : "off"}` }),
          wide ? h("span", { className: "dcu-actionText" }, primary ? `${remaining}%` : "Codex") : null,
        ),
        open ? h("aside", { className: "dcu-pop" },
          h("div", { className: "dcu-popHead" }, h("strong", null, text("Codex 状态与用量", "Codex status & usage")), h("button", { type: "button", className: "dcu-close", onClick: () => setOpen(false), "aria-label": text("关闭", "Close") }, "×")),
          h(Compact, { usage }),
        ) : null,
      );
    }

    function UsageSettings() {
      const usage = useUsage();
      return h("section", { className: "dcu-section" },
        h("div", { className: "dcu-head" }, h("div", null, h("h2", { className: "dcu-title" }, text("Codex 后端、账户与用量", "Codex backend, account & usage")), h("p", { className: "dcu-sub" }, text("控制面由 dsh 插件提供；系统操作由 DSH Desktop 的受控本地桥接执行。", "This control surface is provided by a dsh plugin; system operations use DSH Desktop's restricted local bridge."))), h("button", { type: "button", className: "dcu-refresh", onClick: () => store.refresh() }, text("刷新用量", "Refresh usage"))),
        h(CodexControl),
        usage.error ? h("div", { className: "dcu-error" }, usage.error) : null,
        h("div", { className: "dcu-grid" }, h(RuntimeCard, { usage }), h(AccountCard, { usage }), h(CreditsCard, { usage })),
        usage.buckets?.length ? h("div", null, h("h3", null, text("订阅限额", "Subscription limits")), h("div", { className: "dcu-grid" }, usage.buckets.map((bucket) => h(Bucket, { key: bucket.id, bucket })))) : null,
        h("div", { className: "dcu-grid" }, h(HistoryCard, { account: usage.accountUsage }), h(ThreadsCard, { threads: usage.threadUsage })),
        h("p", { className: "dcu-sub" }, text("百分比是服务端 usedPercent 推导的近似剩余量，不代表可用 Token 的绝对数量。线程数据按 Codex thread ID 汇总，尚不与 dsh 会话强行关联。", "Remaining percentages are derived from server usedPercent and are not absolute token counts. Thread usage is keyed by Codex thread ID and is not forcibly mapped to dsh sessions.")),
        h("p", { className: "dcu-sub" }, text("最后更新：", "Last updated: "), updated(usage.updatedAt)),
      );
    }

    const inject = ["slots", "sessions", "workspaces"];
    function apply(ctx) {
      const linkedSessions = new Map();
      let projectingSessions = false;
      let previousBackend = store.getSnapshot().backend;

      const projectLinkedSessions = () => {
        if (projectingSessions || store.getSnapshot().backend !== "codex") return;
        const snapshot = ctx.sessions.list.getSnapshot();
        const changes = [...linkedSessions.values()].filter((entry) => {
          const summary = snapshot.byId[entry.sessionId];
          return summary && (summary.blank || (entry.title && summary.displayTitle !== entry.title));
        });
        if (!changes.length || typeof ctx.sessions.list.update !== "function") return;
        projectingSessions = true;
        try {
          ctx.sessions.list.update((draft) => {
            for (const entry of changes) {
              const summary = draft.byId[entry.sessionId];
              if (!summary) continue;
              // Codex owns the durable conversation log, so the dsh Host still
              // reports this carrier session as blank. Projecting it as engaged
              // is what lets the standard workspace plugin keep it visible.
              summary.blank = false;
              if (entry.title) {
                summary.title = entry.title;
                summary.displayTitle = entry.title;
              }
            }
          });
        } finally {
          projectingSessions = false;
        }
      };

      const refreshLinkedSessions = async () => {
        const entries = await invokeDesktop("codex_session_index");
        linkedSessions.clear();
        for (const entry of entries || []) linkedSessions.set(entry.sessionId, entry);
        projectLinkedSessions();
        return entries;
      };

      const markCodexSession = async (sessionId, title) => {
        linkedSessions.set(sessionId, { sessionId, title });
        projectLinkedSessions();
        const binding = ctx.sessions.binding(sessionId);
        if (binding && title) {
          try {
            const result = await binding.session.rename(title);
            if (result && result.ok === false) console.warn("[dsh-codex-usage] unable to persist Codex session title", result.error);
          } catch (error) {
            console.warn("[dsh-codex-usage] unable to persist Codex session title", error);
          }
        }
        void refreshLinkedSessions().catch((error) => console.warn("[dsh-codex-usage] unable to refresh Codex session index", error));
      };

      const startCodexSession = async ({ workspaceId, cwd } = {}) => {
        if (typeof ctx.sessions.create !== "function") {
          throw new Error(text("当前 dsh 运行时不支持创建独立会话。", "The current dsh runtime cannot create an independent session."));
        }
        const sessions = ctx.sessions.list.getSnapshot();
        const workspaces = ctx.workspaces.list.getSnapshot();
        const current = sessions.current ? sessions.byId[sessions.current] : null;
        let targetWorkspaceId = workspaceId;
        if (!targetWorkspaceId && sessions.current) {
          targetWorkspaceId = workspaces.items.find((item) => item.sessionIds?.includes(sessions.current))?.workspaceId;
        }
        if (!targetWorkspaceId) {
          const targetPath = cwd || current?.cwd;
          targetWorkspaceId = workspaces.items.find((item) => item.path === targetPath)?.workspaceId || workspaces.recentWorkspaceId;
        }
        const options = targetWorkspaceId ? { workspaceId: targetWorkspaceId } : (cwd || current?.cwd) ? { cwd: cwd || current.cwd } : {};
        const sessionId = await ctx.sessions.create(options);
        ctx.sessions.open(sessionId);
        return sessionId;
      };

      const withSessionActions = (Component) => (props) => h(Component, { ...props, startCodexSession, markCodexSession });
      const CodexComposerWithActions = withSessionActions(CodexComposer);
      const CodexHandoffWithActions = withSessionActions(CodexHandoffAction);

      ctx.effect(() => { store.start(); return () => store.stop(); }, "desktop-codex-usage: live store");
      ctx.effect(() => {
        const originalStartSession = ctx.workspaces.startSession;
        const codexAwareStartSession = function (workspaceId) {
          if (store.getSnapshot().backend !== "codex") return originalStartSession.call(this, workspaceId);
          void startCodexSession({ workspaceId }).catch((error) => console.error("[dsh-codex-usage] unable to create Codex session", error));
        };
        ctx.workspaces.startSession = codexAwareStartSession;
        return () => {
          if (ctx.workspaces.startSession === codexAwareStartSession) ctx.workspaces.startSession = originalStartSession;
        };
      }, "desktop-codex-usage: Codex-aware new session");
      ctx.effect(() => {
        const unsubscribeSessions = ctx.sessions.list.subscribe(projectLinkedSessions);
        const unsubscribeUsage = store.subscribe(() => {
          const backend = store.getSnapshot().backend;
          if (backend === "codex") projectLinkedSessions();
          if (previousBackend === "codex" && backend === "dsh" && typeof ctx.sessions.refresh === "function") {
            void ctx.sessions.refresh();
          }
          previousBackend = backend;
        });
        void refreshLinkedSessions().catch(() => {});
        return () => { unsubscribeSessions(); unsubscribeUsage(); };
      }, "desktop-codex-usage: workspace session projection");
      ctx.slots.inject("sidebar.footer.action", () => ctx.slots.register({
        name: "sidebar.footer.action", id: "desktop-codex-usage", order: 20,
      }, SidebarUsage));
      ctx.slots.inject("settings.section", () => ctx.slots.register({
        name: "settings.section", id: "desktop-codex-usage", order: 20,
        label: () => "Codex", inject: () => ({}),
      }, UsageSettings));
      ctx.slots.inject("conversation.session.header.actions", () => ctx.slots.register({
        name: "conversation.session.header.actions", id: "desktop-codex-handoff", order: 90,
      }, CodexHandoffWithActions));
      ctx.slots.inject("conversation.composer", () => {
        let disposeComposer = null;
        const syncComposer = () => {
          const active = store.getSnapshot().backend === "codex";
          if (active && !disposeComposer) {
            disposeComposer = ctx.slots.register({
              name: "conversation.composer",
              priority: 0,
              select: (owner) => owner.session || { backend: "codex" },
            }, CodexComposerWithActions);
          } else if (!active && disposeComposer) {
            const dispose = disposeComposer;
            disposeComposer = null;
            dispose();
          }
        };
        const unsubscribe = store.subscribe(syncComposer);
        syncComposer();
        return () => {
          unsubscribe();
          disposeComposer?.();
          disposeComposer = null;
        };
      });
    }

    exports.apply = apply;
    exports.inject = inject;
    return module.exports;
  },
});
