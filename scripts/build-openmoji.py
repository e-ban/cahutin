#!/usr/bin/env python3
"""Construit assets/openmoji/ : la banque d'images OpenMoji 17.0 embarquée dans Carnet.

À lancer une fois (le résultat est commité) : python3 scripts/build-openmoji.py
Garde les images couleur SVG sans variantes de couleur de peau ni drapeaux, avec des mots-clés
français (annotations CLDR d'Unicode) et anglais (OpenMoji).
"""
import io, json, urllib.request, zipfile, pathlib

OPENMOJI = "17.0.0"
CLDR = "48.0.0"
OUT = pathlib.Path(__file__).resolve().parent.parent / "assets" / "openmoji"
GROUPS = {
    "smileys-emotion": "visages", "people-body": "personnes", "animals-nature": "animaux",
    "food-drink": "nourriture", "travel-places": "lieux", "activities": "activites",
    "objects": "objets", "symbols": "symboles", "component": "symboles",
    "extras-openmoji": "openmoji", "extras-unicode": "openmoji",
}

def get(url):
    with urllib.request.urlopen(url) as r:
        return r.read()

def main():
    meta = json.loads(get(f"https://raw.githubusercontent.com/hfg-gmuend/openmoji/{OPENMOJI}/data/openmoji.json"))
    svgs = zipfile.ZipFile(io.BytesIO(get(f"https://github.com/hfg-gmuend/openmoji/releases/download/{OPENMOJI}/openmoji-svg-color.zip")))
    base = f"https://raw.githubusercontent.com/unicode-org/cldr-json/{CLDR}/cldr-json"
    fr = json.loads(get(f"{base}/cldr-annotations-full/annotations/fr/annotations.json"))["annotations"]["annotations"]
    frd = json.loads(get(f"{base}/cldr-annotations-derived-full/annotationsDerived/fr/annotations.json"))["annotationsDerived"]["annotations"]
    licence = get(f"https://raw.githubusercontent.com/hfg-gmuend/openmoji/{OPENMOJI}/LICENSE.txt")
    cldr_licence = get(f"https://raw.githubusercontent.com/unicode-org/cldr-json/{CLDR}/LICENSE")

    def french(e):
        for table in (fr, frd):
            for key in (e, e.replace("️", "")):
                if key in table:
                    a = table[key]
                    return list(dict.fromkeys(a.get("tts", []) + a.get("default", [])))
        return []

    index, out = [], io.BytesIO()
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for m in meta:
            if m["skintone"] or m["group"] == "flags":
                continue
            name = f"{m['hexcode']}.svg"
            z.writestr(name, svgs.read(name))
            en = [m["annotation"]] + [t.strip() for t in (m["tags"] + "," + m["openmoji_tags"]).split(",") if t.strip()]
            index.append({"code": m["hexcode"], "group": GROUPS[m["group"]], "fr": french(m["emoji"]), "en": list(dict.fromkeys(en))})
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "openmoji.zip").write_bytes(out.getvalue())
    (OUT / "index.json").write_text(json.dumps(index, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    (OUT / "LICENSE-OpenMoji.txt").write_bytes(licence)
    (OUT / "LICENSE-CLDR.txt").write_bytes(cldr_licence)
    print(f"{len(index)} images")

if __name__ == "__main__":
    main()
