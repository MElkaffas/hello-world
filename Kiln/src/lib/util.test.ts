import { describe, expect, it } from "vitest";
import type { ActionInfo } from "../api";
import { formatBytes, groupConversions, parseTime, previewName, validateOptions, wheelActions } from "./util";

const mk = (kind: string, to: string | undefined, group: "convert" | "tool", recommended = false, cat: ActionInfo["output_category"] = "image", params: ActionInfo["params"] = []): ActionInfo => ({
  action: to ? { kind, to } : { kind },
  label: to ?? kind,
  group,
  output_category: cat,
  recommended,
  combines_inputs: false,
  produces_folder: false,
  engines: [],
  params,
});

describe("util", () => {
  it("formats bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
  });

  it("parses times", () => {
    expect(parseTime("90")).toBe(90);
    expect(parseTime("1:30")).toBe(90);
    expect(parseTime("01:00:01.5")).toBe(3601.5);
    expect(parseTime("")).toBeNull();
    expect(parseTime("abc")).toBeNaN();
    expect(parseTime("1::2")).toBeNaN();
  });

  it("groups conversions with recommended first", () => {
    const g = groupConversions([mk("convert", "Png", "convert"), mk("convert", "Jpg", "convert", true), mk("convert", "Pdf", "convert", false, "pdf"), mk("image_resize", undefined, "tool")]);
    expect(g.map((x) => x.category)).toEqual(["image", "pdf"]);
    expect(g[0].items[0].action.to).toBe("Jpg");
  });

  it("filters by search", () => {
    const g = groupConversions([mk("convert", "Png", "convert"), mk("convert", "Jpg", "convert")], "jp");
    expect(g[0].items.map((i) => i.action.to)).toEqual(["Jpg"]);
  });

  it("wheel excludes tools that need user input", () => {
    const w = wheelActions([mk("convert", "Jpg", "convert", true), mk("image_crop", undefined, "tool", false, "image", ["crop"]), mk("strip_metadata", undefined, "tool")]);
    expect(w.map((a) => a.action.kind)).toEqual(["convert", "strip_metadata"]);
  });

  it("validates options", () => {
    const resize = mk("image_resize", undefined, "tool");
    expect(validateOptions(resize, {})).toMatch(/width/);
    expect(validateOptions(resize, { width: 100 })).toBeNull();
    const trim = mk("trim", undefined, "tool");
    expect(validateOptions(trim, { start: 5, end: 2 })).toMatch(/after/);
    expect(validateOptions(mk("pdf_pages", undefined, "tool"), { pages: "1;2" })).toMatch(/Pages/);
    expect(validateOptions(mk("pdf_pages", undefined, "tool"), { pages: "3,1-2,z" })).toBeNull();
  });

  it("previews names safely", () => {
    expect(previewName("{name}", "Holiday.heic", "jpg")).toBe("Holiday.jpg");
    expect(previewName("{name}-web", "a.png", "webp")).toBe("a-web.webp");
    expect(previewName("../{name}", "a.png", "jpg")).toBe(".._a.jpg");
  });
});
