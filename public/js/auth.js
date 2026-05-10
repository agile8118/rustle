// Shared auth UI: login form, register form, logout button.
(function () {
  async function postJson(url, body) {
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
      credentials: "same-origin",
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
      const err = new Error(data.message || `Request failed: ${res.status}`);
      err.status = res.status;
      throw err;
    }
    return data;
  }

  function showError(el, msg) {
    el.textContent = msg;
    el.hidden = false;
  }

  const loginForm = document.getElementById("login-form");
  if (loginForm) {
    const errEl = document.getElementById("login-error");
    loginForm.addEventListener("submit", async (e) => {
      e.preventDefault();
      errEl.hidden = true;
      const fd = new FormData(loginForm);
      try {
        await postJson("/api/auth/login", {
          email: fd.get("email"),
          password: fd.get("password"),
        });
        window.location.href = "/dashboard";
      } catch (err) {
        showError(errEl, "Invalid email or password.");
      }
    });
  }

  const registerForm = document.getElementById("register-form");
  if (registerForm) {
    const errEl = document.getElementById("register-error");
    registerForm.addEventListener("submit", async (e) => {
      e.preventDefault();
      errEl.hidden = true;
      const fd = new FormData(registerForm);
      try {
        await postJson("/api/auth/register", {
          email: fd.get("email"),
          password: fd.get("password"),
          display_name: fd.get("display_name"),
        });
        window.location.href = "/dashboard";
      } catch (err) {
        showError(errEl, err.message || "Could not create account.");
      }
    });
  }

  const logoutBtn = document.getElementById("logout-btn");
  if (logoutBtn) {
    logoutBtn.addEventListener("click", async () => {
      try {
        await postJson("/api/auth/logout", {});
      } catch (_) { /* ignore */ }
      window.location.href = "/";
    });
  }

  window.RustleApi = {
    async json(method, url, body) {
      const opts = {
        method,
        credentials: "same-origin",
        headers: {},
      };
      if (body !== undefined) {
        opts.headers["Content-Type"] = "application/json";
        opts.body = JSON.stringify(body);
      }
      const res = await fetch(url, opts);
      const data = res.status === 204 ? null : await res.json().catch(() => ({}));
      if (!res.ok) {
        const err = new Error(data && data.message || `Request failed: ${res.status}`);
        err.status = res.status;
        throw err;
      }
      return data;
    },
  };
})();
