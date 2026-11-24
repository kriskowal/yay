import { makeMarshal } from "@endo/marshal";

/** Make an Endo marshal configured for YSON's smallcaps body format. */
export const makeSmallcapsMarshal = (convertValToSlot, convertSlotToVal) =>
  makeMarshal(convertValToSlot, convertSlotToVal, {
    serializeBodyFormat: "smallcaps",
  });
