#!/usr/bin/env python3
"""
export-nodered-flows.py
Exporte les flows Node-RED actuels vers flux-nodered/*.json (un fichier par tab).

Usage :
  python3 contrib/export-nodered-flows.py
  NODERED_URL=http://192.168.1.141:1880 python3 contrib/export-nodered-flows.py

Principe :
  1. GET /flows → tous les nœuds Node-RED actuels
  2. Regroupe par tab (un fichier JSON par tab = par onglet)
  3. Les nœuds globaux (config, mqtt-broker, etc.) sont répartis dans le
     premier tab qui les référence, ou dans un fichier _global.json si aucun.
  4. Écrase les fichiers existants dans flux-nodered/

Workflow recommandé :
  1. Éditer les flows dans l'UI Node-RED (http://192.168.1.141:1880)
  2. Cliquer "Deploy" dans l'UI
  3. Lancer : make export-nodered
  4. git add flux-nodered/ && git commit && git push
  5. Sur le Pi5 : git pull && make deploy-nodered
"""

import json
import os
import glob
import sys
import re
import urllib.request
import urllib.error
import time

NODERED_URL = os.environ.get("NODERED_URL", "http://localhost:1880")
FLOWS_DIR   = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "flux-nodered"))


def get_flows():
    req = urllib.request.Request(f"{NODERED_URL}/flows")
    req.add_header("Accept", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read())
            return data if isinstance(data, list) else data.get("flows", [])
    except urllib.error.URLError as e:
        print(f"ERREUR : impossible de joindre Node-RED ({NODERED_URL}) : {e}")
        sys.exit(1)


def slugify(name):
    """Convertit un nom de tab en nom de fichier safe."""
    s = name.lower().strip()
    s = re.sub(r'[àáâãäå]', 'a', s)
    s = re.sub(r'[èéêë]', 'e', s)
    s = re.sub(r'[ìíîï]', 'i', s)
    s = re.sub(r'[òóôõö]', 'o', s)
    s = re.sub(r'[ùúûü]', 'u', s)
    s = re.sub(r'[ç]', 'c', s)
    s = re.sub(r'[^a-z0-9]+', '-', s)
    s = s.strip('-')
    return s or "tab"


def wait_for_nodered(max_wait=10):
    print(f"Connexion à Node-RED ({NODERED_URL})...", end="", flush=True)
    for _ in range(max_wait):
        try:
            urllib.request.urlopen(f"{NODERED_URL}", timeout=2)
            print(" OK")
            return
        except Exception:
            print(".", end="", flush=True)
            time.sleep(1)
    print("\nErreur : Node-RED n'a pas répondu.")
    sys.exit(1)


def main():
    print("=" * 60)
    print("Export des flows Node-RED → git")
    print(f"  Source : {NODERED_URL}")
    print(f"  Cible  : {FLOWS_DIR}")
    print("=" * 60)

    wait_for_nodered()

    print("\nRécupération des flows depuis Node-RED...")
    all_nodes = get_flows()
    print(f"  {len(all_nodes)} nœuds récupérés")

    # Séparer tabs, nœuds de flow, nœuds de config
    tabs        = [n for n in all_nodes if n.get("type") == "tab"]
    config_nodes = [n for n in all_nodes if not n.get("z") and n.get("type") != "tab"]
    flow_nodes  = [n for n in all_nodes if n.get("z")]

    print(f"  {len(tabs)} tabs, {len(flow_nodes)} nœuds de flow, {len(config_nodes)} nœuds de config")

    if not tabs:
        print("Aucun tab trouvé — rien à exporter.")
        sys.exit(0)

    # Regrouper les nœuds de flow par tab ID
    nodes_by_tab = {tab["id"]: [] for tab in tabs}
    for node in flow_nodes:
        tab_id = node.get("z")
        if tab_id in nodes_by_tab:
            nodes_by_tab[tab_id].append(node)

    # Déterminer quels config nodes sont utilisés par chaque tab
    # (on répartit les config nodes dans tous les tabs qui les référencent)
    config_ids = {n["id"] for n in config_nodes}

    def find_refs(node):
        """Trouve tous les IDs référencés dans un nœud."""
        refs = set()
        for val in node.values():
            if isinstance(val, str) and val in config_ids:
                refs.add(val)
            elif isinstance(val, list):
                for item in val:
                    if isinstance(item, str) and item in config_ids:
                        refs.add(item)
        return refs

    # Pour chaque tab, collecter les config nodes nécessaires
    config_by_tab = {tab["id"]: set() for tab in tabs}
    for tab in tabs:
        for node in nodes_by_tab[tab["id"]]:
            refs = find_refs(node)
            config_by_tab[tab["id"]].update(refs)

    # Résoudre les dépendances transitives (config node peut référencer d'autres config nodes)
    config_node_map = {n["id"]: n for n in config_nodes}
    for tab in tabs:
        extra = set()
        for cfg_id in config_by_tab[tab["id"]]:
            if cfg_id in config_node_map:
                extra.update(find_refs(config_node_map[cfg_id]))
        config_by_tab[tab["id"]].update(extra)

    # Écrire un fichier par tab
    os.makedirs(FLOWS_DIR, exist_ok=True)

    # Garder la liste des fichiers existants pour nettoyer les supprimés
    existing_files = set(glob.glob(os.path.join(FLOWS_DIR, "*.json")))
    written_files  = set()

    # Noms de fichier existants → garder la correspondance tab↔fichier si possible
    # En cherchant dans les anciens fichiers le tab ID
    existing_tab_to_file = {}
    for fpath in existing_files:
        try:
            with open(fpath, encoding="utf-8") as f:
                nodes = json.load(f)
            for n in nodes:
                if n.get("type") == "tab":
                    existing_tab_to_file[n["id"]] = os.path.basename(fpath)
        except Exception:
            pass

    # Compter les tabs sans nœuds (tabs vides)
    skipped_empty = 0
    tab_filenames = {}  # tab_id → filename (pour détection de conflits)

    for tab in tabs:
        tab_id    = tab["id"]
        tab_name  = tab.get("label", tab_id)
        flow_nds  = nodes_by_tab[tab_id]
        cfg_nds   = [config_node_map[cid] for cid in config_by_tab[tab_id] if cid in config_node_map]

        # Sauter les tabs vides (pas de nœuds de flow et pas de config propres)
        if not flow_nds and not cfg_nds:
            skipped_empty += 1
            continue

        # Déterminer le nom de fichier
        if tab_id in existing_tab_to_file:
            # Conserver le nom de fichier existant (évite les renames inutiles)
            fname = existing_tab_to_file[tab_id]
        else:
            base = slugify(tab_name)
            fname = f"{base}.json"
            # Éviter les collisions de noms
            counter = 2
            while fname in tab_filenames.values():
                fname = f"{base}-{counter}.json"
                counter += 1

        tab_filenames[tab_id] = fname
        fpath = os.path.join(FLOWS_DIR, fname)

        # Assembler : tab + config nodes + flow nodes
        file_nodes = [tab] + cfg_nds + flow_nds

        with open(fpath, "w", encoding="utf-8") as f:
            json.dump(file_nodes, f, indent=2, ensure_ascii=False)
            f.write("\n")

        written_files.add(fpath)
        print(f"  [OK]  {fname} ({len(file_nodes)} nœuds — tab '{tab_name}')")

    # Config nodes orphelins (non référencés par aucun tab)
    all_used_cfg = set()
    for used in config_by_tab.values():
        all_used_cfg.update(used)
    orphan_cfg = [n for n in config_nodes if n["id"] not in all_used_cfg]
    if orphan_cfg:
        fname = "_global.json"
        fpath = os.path.join(FLOWS_DIR, fname)
        with open(fpath, "w", encoding="utf-8") as f:
            json.dump(orphan_cfg, f, indent=2, ensure_ascii=False)
            f.write("\n")
        written_files.add(fpath)
        print(f"  [OK]  {fname} ({len(orphan_cfg)} nœuds de config orphelins)")

    # Fichiers supprimés (dans git mais plus dans Node-RED)
    removed = existing_files - written_files
    if removed:
        print(f"\nFichiers à supprimer de git (tabs supprimés dans Node-RED) :")
        for fpath in sorted(removed):
            print(f"  [RM]  {os.path.basename(fpath)}")
            os.remove(fpath)

    if skipped_empty:
        print(f"\n  ({skipped_empty} tabs vides ignorés)")

    print(f"\n{'='*60}")
    print(f"✓ Export terminé : {len(written_files)} fichiers dans flux-nodered/")
    print(f"\nÉtapes suivantes :")
    print(f"  git add flux-nodered/")
    print(f"  git diff --stat flux-nodered/")
    print(f"  git commit -m 'feat(nodered): export flows depuis Node-RED'")
    print(f"  git push -u origin <branche>")
    print("=" * 60)


if __name__ == "__main__":
    main()
