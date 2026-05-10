(function () {
  const root = document.querySelector(".board-page");
  if (!root) return;
  const boardId = root.dataset.boardId;
  const kanbanEl = document.getElementById("kanban");
  const titleEl = document.getElementById("board-title");
  let state = null;

  async function load() {
    state = await RustleApi.json("GET", `/api/boards/${boardId}`);
    titleEl.textContent = state.title;
    render();
  }

  function render() {
    kanbanEl.innerHTML = "";
    for (const col of state.columns) {
      kanbanEl.appendChild(renderColumn(col));
    }
  }

  function renderColumn(col) {
    const colEl = document.createElement("section");
    colEl.className = "column";
    colEl.dataset.colId = col.id;
    colEl.innerHTML = `
      <header class="column-head">
        <h3 contenteditable spellcheck="false">${escapeHtml(col.title)}</h3>
        <div class="col-actions">
          <button type="button" data-action="delete-col" title="Delete column">×</button>
        </div>
      </header>
      <ul class="cards-list" data-col-id="${col.id}"></ul>
      <form class="card-add" data-col-id="${col.id}">
        <input type="text" name="title" placeholder="+ Add card" maxlength="200" required />
      </form>
    `;
    const list = colEl.querySelector(".cards-list");
    for (const card of col.cards) list.appendChild(renderCard(card));

    const head = colEl.querySelector("h3");
    head.addEventListener("blur", async () => {
      const newTitle = head.textContent.trim();
      if (newTitle && newTitle !== col.title) {
        try {
          await RustleApi.json("PATCH", `/api/columns/${col.id}`, { title: newTitle });
          col.title = newTitle;
        } catch (e) { head.textContent = col.title; }
      } else if (!newTitle) {
        head.textContent = col.title;
      }
    });
    head.addEventListener("keydown", (e) => {
      if (e.key === "Enter") { e.preventDefault(); head.blur(); }
    });

    colEl.querySelector('[data-action="delete-col"]').addEventListener("click", async () => {
      if (!confirm(`Delete column "${col.title}" and all its cards?`)) return;
      await RustleApi.json("DELETE", `/api/columns/${col.id}`);
      await load();
    });

    const form = colEl.querySelector(".card-add");
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const fd = new FormData(form);
      const title = fd.get("title");
      if (!title) return;
      try {
        await RustleApi.json("POST", `/api/columns/${col.id}/cards`, { title });
        form.reset();
        await load();
      } catch (e) { alert(e.message); }
    });

    list.addEventListener("dragover", (e) => { e.preventDefault(); list.classList.add("drop-target"); });
    list.addEventListener("dragleave", () => list.classList.remove("drop-target"));
    list.addEventListener("drop", async (e) => {
      e.preventDefault();
      list.classList.remove("drop-target");
      const cardId = e.dataTransfer.getData("text/plain");
      if (!cardId) return;
      // Determine drop position based on cursor
      const cards = [...list.querySelectorAll(".card-item:not(.dragging)")];
      const after = cards.find(el => e.clientY < el.getBoundingClientRect().top + el.offsetHeight / 2);
      const targetIndex = after ? cards.indexOf(after) : cards.length;
      try {
        await RustleApi.json("POST", `/api/cards/${cardId}/move`, {
          column_id: col.id,
          position: targetIndex,
        });
        await load();
      } catch (err) { alert(err.message); }
    });

    return colEl;
  }

  function renderCard(card) {
    const li = document.createElement("li");
    li.className = "card-item";
    li.draggable = true;
    li.dataset.cardId = card.id;
    li.innerHTML = `<div>${escapeHtml(card.title)}</div>${card.description ? `<div class="desc">${escapeHtml(card.description)}</div>` : ""}`;
    li.addEventListener("dragstart", (e) => {
      li.classList.add("dragging");
      e.dataTransfer.setData("text/plain", card.id);
      e.dataTransfer.effectAllowed = "move";
    });
    li.addEventListener("dragend", () => li.classList.remove("dragging"));
    li.addEventListener("dblclick", async () => {
      const t = prompt("Rename card", card.title);
      if (t && t !== card.title) {
        try {
          await RustleApi.json("PATCH", `/api/cards/${card.id}`, { title: t });
          await load();
        } catch (e) { alert(e.message); }
      }
    });
    return li;
  }

  function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, c => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    })[c]);
  }

  document.getElementById("add-column-btn").addEventListener("click", async () => {
    const title = prompt("Column name");
    if (!title) return;
    await RustleApi.json("POST", `/api/boards/${boardId}/columns`, { title });
    await load();
  });

  document.getElementById("rename-board-btn").addEventListener("click", async () => {
    const t = prompt("Rename board", state.title);
    if (t && t !== state.title) {
      await RustleApi.json("PATCH", `/api/boards/${boardId}`, { title: t });
      await load();
    }
  });

  document.getElementById("delete-board-btn").addEventListener("click", async () => {
    if (!confirm("Delete this board and everything on it?")) return;
    await RustleApi.json("DELETE", `/api/boards/${boardId}`);
    window.location.href = "/dashboard";
  });

  load().catch(err => {
    kanbanEl.innerHTML = `<p class="error">${err.message || "Could not load board."}</p>`;
  });
})();
