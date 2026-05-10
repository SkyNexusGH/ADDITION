// Nexus Mods Single Sign-On flow.
// Reference: https://github.com/Nexus-Mods/sso-integration-demo
//
// Protocol (v2):
//   1. Open wss://sso.nexusmods.com
//   2. Send {"id": uuid, "token": null|prev_token, "protocol": 2}
//   3. Receive {"success": true, "data": {"connection_token": "..."}}
//      → store the connection_token (so future re-auths skip the consent screen)
//   4. Open https://www.nexusmods.com/sso?id={uuid}&application={slug} in the user's browser
//   5. Wait for {"success": true, "data": {"api_key": "..."}}
//   6. Close socket; store api_key
//
// The slug is registered on nexusmods.com under My Account → API. Until ADDITION
// is registered, this will fail with a "no application matched" message — that
// is intentional and the right behaviour pre-launch.

import { dbq } from "./db";
import { api } from "./tauri";

const SSO_WS_URL = "wss://sso.nexusmods.com";
const SSO_WEB_URL = "https://www.nexusmods.com/sso";
// TODO: switch to "addition" once the slug is registered (ask in #api on the
// Nexus Mods Discord). Using "vortex" — a known-good slug — for testing so the
// SSO flow works end-to-end. The Nexus consent screen will read "Vortex" until
// the swap; the API key returned is still scoped to the user's own account.
const APP_SLUG = "vortex";
const TIMEOUT_MS = 5 * 60 * 1000; // 5 minutes for the user to approve

export interface NexusLoginResult {
  apiKey: string;
}

function uuid(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return "10000000-1000-4000-8000-100000000000".replace(/[018]/g, (c) =>
    (
      Number(c) ^
      (crypto.getRandomValues(new Uint8Array(1))[0] & (15 >> (Number(c) / 4)))
    ).toString(16)
  );
}

export function nexusLogin(
  onProgress?: (msg: string) => void
): Promise<NexusLoginResult> {
  return new Promise(async (resolve, reject) => {
    const id = uuid();
    const previousToken = await dbq.getSetting("nexus_sso_token");

    let resolved = false;
    let browserOpened = false;
    const ws = new WebSocket(SSO_WS_URL);

    const fail = (err: Error) => {
      if (resolved) return;
      resolved = true;
      try {
        ws.close();
      } catch {}
      reject(err);
    };

    const success = (apiKey: string) => {
      if (resolved) return;
      resolved = true;
      try {
        ws.close();
      } catch {}
      resolve({ apiKey });
    };

    const timer = setTimeout(
      () => fail(new Error("Nexus sign-in timed out (no response in 5 minutes).")),
      TIMEOUT_MS
    );

    ws.onopen = () => {
      onProgress?.("Connecting to Nexus…");
      ws.send(
        JSON.stringify({
          id,
          token: previousToken,
          protocol: 2,
        })
      );
    };

    ws.onerror = () => {
      clearTimeout(timer);
      fail(new Error("Could not reach the Nexus SSO server."));
    };

    ws.onclose = () => {
      clearTimeout(timer);
      if (!resolved) {
        fail(new Error("Nexus SSO connection closed before completion."));
      }
    };

    ws.onmessage = async (evt) => {
      let payload: any;
      try {
        payload = JSON.parse(evt.data);
      } catch {
        return;
      }

      if (payload.success === false) {
        return fail(new Error(payload.error || "Nexus SSO returned an error."));
      }

      const data = payload.data ?? {};

      if (data.connection_token) {
        await dbq.setSetting("nexus_sso_token", data.connection_token);
        if (!browserOpened) {
          browserOpened = true;
          onProgress?.("Opening Nexus in your browser — approve the request to continue.");
          try {
            await api.openPath(
              `${SSO_WEB_URL}?id=${encodeURIComponent(id)}&application=${encodeURIComponent(
                APP_SLUG
              )}`
            );
          } catch (e: any) {
            return fail(new Error(`Could not open browser: ${e?.toString?.()}`));
          }
        }
        return;
      }

      if (data.api_key) {
        clearTimeout(timer);
        await dbq.setSetting("api_nexus", data.api_key);
        await dbq.setSetting("nexus_signed_in_at", new Date().toISOString());
        onProgress?.("Signed in.");
        return success(data.api_key);
      }
    };
  });
}

export async function nexusLogout() {
  await dbq.setSetting("api_nexus", "");
  await dbq.setSetting("nexus_sso_token", "");
  await dbq.setSetting("nexus_signed_in_at", "");
}

export async function nexusStatus(): Promise<{
  signedIn: boolean;
  signedInAt: string | null;
}> {
  const key = await dbq.getSetting("api_nexus");
  const at = await dbq.getSetting("nexus_signed_in_at");
  return {
    signedIn: !!key,
    signedInAt: at || null,
  };
}
