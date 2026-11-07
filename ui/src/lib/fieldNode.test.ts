import { getSchema } from "@tiptap/core";
import Document from "@tiptap/extension-document";
import Paragraph from "@tiptap/extension-paragraph";
import Text from "@tiptap/extension-text";
import { expect, test } from "vitest";
import { ClassField, plainText } from "./fieldNode";

test("a copied pastille keeps its braces, so it pastes back as a field", () => {
  const schema = getSchema([Document, Paragraph, Text, ClassField]);
  const doc = schema.nodeFromJSON({
    type: "doc",
    content: [
      { type: "paragraph", content: [{ type: "text", text: "Bonjour " }, { type: "classField", attrs: { key: "prenom" } }, { type: "text", text: " !" }] },
      { type: "paragraph", content: [{ type: "classField", attrs: { key: "age" } }] },
    ],
  });
  expect(plainText(doc.content)).toBe("Bonjour {prenom} !\n{age}");
});
