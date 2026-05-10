(function () {
  const dialog = document.getElementById("new-board-dialog");
  const openBtn = document.getElementById("new-board-btn");
  const cancelBtn = document.getElementById("new-board-cancel");
  const form = document.getElementById("new-board-form");
  const errEl = document.getElementById("new-board-error");

  if (!openBtn) return;

  openBtn.addEventListener("click", () => {
    errEl.hidden = true;
    form.reset();
    dialog.showModal();
  });
  cancelBtn.addEventListener("click", () => dialog.close());

  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    errEl.hidden = true;
    const title = (new FormData(form)).get("title");
    try {
      const board = await RustleApi.json("POST", "/api/boards", { title });
      window.location.href = `/board/${board.id}`;
    } catch (err) {
      errEl.textContent = err.message || "Could not create board.";
      errEl.hidden = false;
    }
  });
})();
