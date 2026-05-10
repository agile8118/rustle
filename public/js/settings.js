(function () {
  const form = document.getElementById("change-password-form");
  if (!form) return;
  const errEl = document.getElementById("cp-error");
  const okEl = document.getElementById("cp-ok");

  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    errEl.hidden = true; okEl.hidden = true;
    const fd = new FormData(form);
    try {
      await RustleApi.json("PATCH", "/api/auth/password", {
        current: fd.get("current"),
        new: fd.get("new"),
      });
      okEl.hidden = false;
      form.reset();
    } catch (err) {
      errEl.textContent = err.message || "Could not update password.";
      errEl.hidden = false;
    }
  });
})();
