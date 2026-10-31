#!/usr/bin/env python3
"""Construit assets/openmoji/ : la banque d'images OpenMoji 17.0 embarquée dans Carnet.

À lancer une fois (le résultat est commité) : python3 scripts/build-openmoji.py
Garde les images couleur SVG sans drapeaux, avec des mots-clés français (annotations CLDR d'Unicode) et
anglais (OpenMoji). Les variantes de couleur de peau d'une seule teinte sont dans l'archive, hors de
l'index : `tones.json` donne pour chaque image ses cinq teintes, de la plus claire à la plus foncée. Chaque
image de l'index a aussi sa version au trait noir, à colorier : `contour/<code>.svg`.
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
    black = zipfile.ZipFile(io.BytesIO(get(f"https://github.com/hfg-gmuend/openmoji/releases/download/{OPENMOJI}/openmoji-svg-black.zip")))
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

    index, tones, out = [], {}, io.BytesIO()
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for m in meta:
            if m["group"] == "flags":
                continue
            if m["skintone"]:
                if m["skintone_combination"] == "single":
                    z.writestr(f"{m['hexcode']}.svg", svgs.read(f"{m['hexcode']}.svg"))
                    tones.setdefault(m["skintone_base_hexcode"], [None] * 5)[int(m["skintone"]) - 1] = m["hexcode"]
                continue
            name = f"{m['hexcode']}.svg"
            z.writestr(name, svgs.read(name))
            z.writestr(f"contour/{name}", black.read(name))
            en = [m["annotation"]] + [t.strip() for t in (m["tags"] + "," + m["openmoji_tags"]).split(",") if t.strip()]
            index.append({"code": m["hexcode"], "group": GROUPS[m["group"]], "fr": french(m["emoji"]), "en": list(dict.fromkeys(en))})
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "openmoji.zip").write_bytes(out.getvalue())
    (OUT / "index.json").write_text(json.dumps(index, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    known = {e["code"] for e in index}
    tones = {base: codes for base, codes in sorted(tones.items()) if base in known and all(codes)}
    (OUT / "tones.json").write_text(json.dumps(tones, separators=(",", ":")), encoding="utf-8")
    (OUT / "LICENSE-OpenMoji.txt").write_bytes(licence)
    (OUT / "LICENSE-CLDR.txt").write_bytes(cldr_licence)
    print(f"{len(index)} images, {len(tones)} en cinq teintes")

if __name__ == "__main__":
    main()
