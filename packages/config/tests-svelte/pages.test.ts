import { fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import { describe, expect, test } from "vitest";

import {
  BackupSettingsPage,
  RestoreSettingsPage,
  StorageSettingsPage,
} from "../src/poodle.ts";
import {
  CONFIG_BACKUP_CREATE_COMMAND,
  CONFIG_RESTORE_ADAPTER_EXECUTE_COMMAND,
  CONFIG_RESTORE_EXECUTE_COMMAND,
  CONFIG_RESTORE_INSPECT_COMMAND,
  CONFIG_RESTORE_PLAN_COMMAND,
  CONFIG_RESTORE_RECOVER_COMMAND,
  CONFIG_STORAGE_INSPECT_COMMAND,
  type ConfigOperationsSnapshot,
} from "../src/index.ts";
import { fixture, pageFixture } from "./support.ts";

describe("public Poodle config pages", () => {
  test("mounts exact storage identity, roots, and conflict evidence", async () => {
    const { client, snapshot, transport } = pageFixture();
    const mounted = render(StorageSettingsPage, {
      client,
      initialSnapshot: snapshot,
      nextRequestId: () => "request:mounted-storage",
    });

    expect(
      screen.getAllByText("audio.infiniteloop.soundcheck"),
    ).toHaveLength(2);
    expect(
      screen.getByText(
        "/Users/tom/Library/Caches/audio.infiniteloop.soundcheck",
      ),
    ).toBeTruthy();
    const select = screen.getByLabelText("Storage profile");
    await fireEvent.change(select, {
      target: { value: "unified-app-root-v1" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Inspect change" }));
    expect(await screen.findByText("target contains different bytes")).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Confirm transition" }),
    ).toBeNull();
    expect(transport.calls).toEqual([CONFIG_STORAGE_INSPECT_COMMAND]);

    mounted.unmount();
    expect(transport.calls).toHaveLength(1);
  });

  test("mounts pending, locked, corrupt, foreign, and unknown backup states", () => {
    const { client, snapshot } = pageFixture();
    render(BackupSettingsPage, {
      client,
      initialSnapshot: snapshot,
      nextRequestId: () => "request:mounted-backup",
    });

    expect(screen.getByText("Unpublished configuration changes")).toBeTruthy();
    for (const state of ["locked", "corrupt", "foreign", "unknown"]) {
      expect(screen.getByRole("rowheader", { name: state })).toBeTruthy();
    }
    expect(
      screen.getByRole("button", { name: "Flush and create" }),
    ).toBeTruthy();
  });

  test("explicit flush confirmation reaches only backup create", async () => {
    const { client, snapshot, transport } = pageFixture();
    render(BackupSettingsPage, {
      client,
      initialSnapshot: snapshot,
      nextRequestId: () => "request:flush-backup",
    });

    await fireEvent.click(
      screen.getByRole("button", { name: "Flush and create" }),
    );
    await fireEvent.click(
      screen.getByRole("button", { name: "Flush and create backup" }),
    );
    expect(await screen.findByText(/Backup published to/)).toBeTruthy();
    expect(transport.calls).toEqual([CONFIG_BACKUP_CREATE_COMMAND]);
  });

  test("requires explicit choices before issuing a digest-bound restore plan", async () => {
    const { client, snapshot, transport } = pageFixture();
    render(RestoreSettingsPage, {
      client,
      initialSnapshot: snapshot,
      nextRequestId: () => "request:restore-page",
    });

    await fireEvent.click(screen.getByRole("button", { name: "Inspect archive" }));
    expect(await screen.findByText("Migration required (1 → 2)")).toBeTruthy();
    expect(screen.getByText("Archive exclusions")).toBeTruthy();
    expect(
      (screen.getByRole("button", { name: "Review exact plan" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);

    const archiveChoice = within(
      screen.getByRole("radiogroup", {
        name: "Restore choice for app.preferences",
      }),
    ).getByRole("radio", { name: "Use archive" }) as HTMLInputElement;
    await waitFor(() => expect(archiveChoice.disabled).toBe(false));
    await fireEvent.click(archiveChoice);
    for (const domainId of ["app.database", "app.future"]) {
      await fireEvent.click(
        within(
          screen.getByRole("radiogroup", {
            name: `Restore choice for ${domainId}`,
          }),
        ).getByRole("radio", { name: "Keep current" }),
      );
    }
    const review = screen.getByRole("button", {
      name: "Review exact plan",
    }) as HTMLButtonElement;
    await waitFor(() => expect(review.disabled).toBe(false));
    await fireEvent.click(review);

    expect(
      await screen.findByRole("region", { name: "Exact restore plan" }),
    ).toBeTruthy();
    expect(
      screen.getByText(
        "8888888888888888888888888888888888888888888888888888888888888888",
      ),
    ).toBeTruthy();
    expect(transport.calls).toEqual([
      CONFIG_RESTORE_INSPECT_COMMAND,
      CONFIG_RESTORE_PLAN_COMMAND,
    ]);
  });

  test("publication remains host-owned when the settings view closes", async () => {
    const { client, snapshot, transport } = pageFixture();
    transport.hold(CONFIG_RESTORE_EXECUTE_COMMAND);
    const mounted = render(RestoreSettingsPage, {
      client,
      initialSnapshot: snapshot,
      nextRequestId: () => "request:host-owned-restore",
    });

    await fireEvent.click(screen.getByRole("button", { name: "Inspect archive" }));
    const archiveChoice = within(
      await screen.findByRole("radiogroup", {
        name: "Restore choice for app.preferences",
      }),
    ).getByRole("radio", { name: "Use archive" }) as HTMLInputElement;
    await waitFor(() => expect(archiveChoice.disabled).toBe(false));
    await fireEvent.click(archiveChoice);
    for (const domainId of ["app.database", "app.future"]) {
      await fireEvent.click(
        within(
          screen.getByRole("radiogroup", {
            name: `Restore choice for ${domainId}`,
          }),
        ).getByRole("radio", { name: "Keep current" }),
      );
    }
    const review = screen.getByRole("button", {
      name: "Review exact plan",
    }) as HTMLButtonElement;
    await waitFor(() => expect(review.disabled).toBe(false));
    await fireEvent.click(review);
    await fireEvent.click(await screen.findByRole("button", { name: "Restore selected domains…" }));
    await fireEvent.click(screen.getByRole("button", { name: "Publish restore" }));
    expect(
      await screen.findByText("Closing this view does not cancel staging, safety backup, publication, rollback, or recovery."),
    ).toBeTruthy();
    mounted.unmount();
    transport.release(CONFIG_RESTORE_EXECUTE_COMMAND);

    expect(transport.calls).toContain(CONFIG_RESTORE_EXECUTE_COMMAND);
  });

  test.each([
    ["locked", fixture.restoreInspectionStates[1], "Archive locked"],
    ["corrupt", fixture.restoreInspectionStates[2], "archiveCorrupt"],
    ["future", fixture.restoreInspectionStates[3], "archiveFutureVersion"],
  ])("mounts %s archive inspection state", async (_name, outcome, evidence) => {
    const { client, snapshot, transport } = pageFixture();
    transport.responses.set(CONFIG_RESTORE_INSPECT_COMMAND, outcome);
    render(RestoreSettingsPage, {
      client,
      initialSnapshot: snapshot,
      nextRequestId: () => "request:inspection-state",
    });

    await fireEvent.click(screen.getByRole("button", { name: "Inspect archive" }));
    expect(await screen.findByText(new RegExp(evidence))).toBeTruthy();
    expect(transport.calls).toEqual([CONFIG_RESTORE_INSPECT_COMMAND]);
  });

  test("keeps custom-adapter execution separate from the ordinary plan", async () => {
    const { client, snapshot, transport } = pageFixture();
    render(RestoreSettingsPage, {
      client,
      initialSnapshot: snapshot,
      nextRequestId: () => "request:adapter-restore",
    });

    await fireEvent.click(screen.getByRole("button", { name: "Inspect archive" }));
    await fireEvent.click(
      await screen.findByRole("button", { name: "Restore with adapter…" }),
    );
    await fireEvent.click(
      screen.getByRole("button", { name: "Run adapter restore" }),
    );

    expect(await screen.findByText("Adapter restore receipt")).toBeTruthy();
    expect(transport.calls).toEqual([
      CONFIG_RESTORE_INSPECT_COMMAND,
      CONFIG_RESTORE_ADAPTER_EXECUTE_COMMAND,
    ]);
  });

  test("stale restore planning returns to inspected choices without publication", async () => {
    const { client, snapshot, transport } = pageFixture();
    transport.responses.set(
      CONFIG_RESTORE_PLAN_COMMAND,
      fixture.restorePlanStates[1],
    );
    render(RestoreSettingsPage, {
      client,
      initialSnapshot: snapshot,
      nextRequestId: () => "request:stale-restore",
    });

    await inspectAndChooseFixtureDomains();
    await fireEvent.click(
      screen.getByRole("button", { name: "Review exact plan" }),
    );

    expect(await screen.findByText(/restorePlanStale/)).toBeTruthy();
    expect(
      screen.queryByRole("region", { name: "Exact restore plan" }),
    ).toBeNull();
    expect(transport.calls).toEqual([
      CONFIG_RESTORE_INSPECT_COMMAND,
      CONFIG_RESTORE_PLAN_COMMAND,
    ]);
  });

  test("recovery-required snapshot gates ordinary settings work", async () => {
    const { client, transport } = pageFixture();
    const recovery = fixture.restoreExecutionStates[2];
    if (recovery.status !== "recoveryRequired") throw new Error("fixture drift");
    render(RestoreSettingsPage, {
      client,
      initialSnapshot: structuredClone(
        recovery.snapshot,
      ) as ConfigOperationsSnapshot,
      nextRequestId: () => "request:recovery-gate",
    });

    expect(screen.getByText("Recovery required")).toBeTruthy();
    expect(screen.queryByLabelText("Backup archive")).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: "Run recovery" }));
    await fireEvent.click(
      screen.getByRole("button", { name: "Confirm recovery" }),
    );

    expect(await screen.findByText("Recovery receipt")).toBeTruthy();
    expect(transport.calls).toEqual([CONFIG_RESTORE_RECOVER_COMMAND]);
  });
});

async function inspectAndChooseFixtureDomains(): Promise<void> {
  await fireEvent.click(screen.getByRole("button", { name: "Inspect archive" }));
  const archiveChoice = within(
    await screen.findByRole("radiogroup", {
      name: "Restore choice for app.preferences",
    }),
  ).getByRole("radio", { name: "Use archive" }) as HTMLInputElement;
  await waitFor(() => expect(archiveChoice.disabled).toBe(false));
  await fireEvent.click(archiveChoice);
  for (const domainId of ["app.database", "app.future"]) {
    await fireEvent.click(
      within(
        screen.getByRole("radiogroup", {
          name: `Restore choice for ${domainId}`,
        }),
      ).getByRole("radio", { name: "Keep current" }),
    );
  }
  const review = screen.getByRole("button", {
    name: "Review exact plan",
  }) as HTMLButtonElement;
  await waitFor(() => expect(review.disabled).toBe(false));
}
