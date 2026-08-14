const invoke = window.__TAURI__.core.invoke;
const show = (value) => {
  document.getElementById("evidence").textContent = JSON.stringify(value, null, 2);
};
const call = (command) => invoke(command).then(show).catch((error) => show({ error: String(error) }));
document.getElementById("open").onclick = () => call("open_transfer_session");
document.getElementById("close").onclick = () => call("close_transfer_sessions");
document.getElementById("install").onclick = () => call("attempt_install");
document.getElementById("relaunch").onclick = () => call("request_relaunch");
document.getElementById("signin").onclick = () => {
  show({ signIn: "waiting - approve in the browser tab that opens" });
  call("attempt_sign_in");
};
// The relaunch answer is read on start, because the process that asked
// for it is gone. Reaching this line at all is half the evidence.
Promise.all([invoke("proof_state"), invoke("relaunch_state")])
  .then(([state, relaunch]) => show({ ...state, ...relaunch }))
  .catch((error) => show({ error: String(error) }));
