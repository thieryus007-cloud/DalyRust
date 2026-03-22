#!/usr/bin/env python3
"""
deploy-nodered-flows.py
Déploie les flows Node-RED depuis flux-nodered/*.json vers Node-RED via l'API REST.

Usage :
  python3 contrib/deploy-nodered-flows.py
  NODERED_URL=http://192.168.1.141:1880 python3 contrib/deploy-nodered-flows.py

Principe :
  1. GET /flows          → récupère tous les flows actuels de Node-RED
  2. Lit flux-nodered/*.json → nouveaux flows depuis git
  3. Fusionne : les nœuds git écrasent/complètent les nœuds existants (par ID)
  4. POST /flows         → déploie (équivalent du bouton "Deploy" dans l'UI)

Pas de redémarrage Docker nécessaire.
"""

import json
import os
import glob
import sys
import urllib.request
import urllib.error
import time

NODERED_URL = os.environ.get("NODERED_URL", "http://localhost:1880")
FLOWS_DIR   = os.path.join(os.path.dirname(__file__), "..", "flux-nodered")


def get_flows():
    """Récupère tous les flows actuels depuis Node-RED."""
    req = urllib.request.Request(f"{NODERED_URL}/flows")
    req.add_header("Accept", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read())
            # v1 API → array, v2 API → {"flows": [...]}
            return data if isinstance(data, list) else data.get("flows", [])
    except urllib.error.URLError as e:
        print(f"ERREUR : impossible de joindre Node-RED ({NODERED_URL}) : {e}")
        print("  → Vérifier que Node-RED est démarré (make up)")
        sys.exit(1)


def post_flows(flows):
    """Envoie les flows fusionnés à Node-RED et déclenche un déploiement complet."""
    body = json.dumps(flows).encode("utf-8")
    req  = urllib.request.Request(f"{NODERED_URL}/flows", data=body, method="POST")
    req.add_header("Content-Type", "application/json")
    req.add_header("Node-RED-Deployment-Type", "full")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return resp.status
    except urllib.error.HTTPError as e:
        print(f"ERREUR HTTP {e.code} lors du déploiement : {e.read().decode()}")
        sys.exit(1)
    except urllib.error.URLError as e:
        print(f"ERREUR réseau lors du déploiement : {e}")
        sys.exit(1)


def load_git_flows():
    """Lit tous les *.json depuis flux-nodered/ et fusionne en une seule liste dédupliquée."""
    pattern = os.path.join(FLOWS_DIR, "*.json")
    files   = sorted(glob.glob(pattern))
    if not files:
        print(f"ERREUR : aucun fichier .json dans {FLOWS_DIR}")
        sys.exit(1)

    nodes_by_id = {}  # déduplication par ID (dernier lu gagne)
    for fpath in files:
        fname = os.path.basename(fpath)
        try:
            with open(fpath, encoding="utf-8") as f:
                nodes = json.load(f)
            if not isinstance(nodes, list):
                print(f"  [SKIP] {fname} : format inattendu (pas un tableau)")
                continue
            for node in nodes:
                if "id" in node:
                    nodes_by_id[node["id"]] = node
            print(f"  [OK]   {fname} ({len(nodes)} nœuds)")
        except Exception as e:
            print(f"  [ERR]  {fname} : {e}")

    return nodes_by_id


def merge(current_flows, git_nodes_by_id):
    """
    Stratégie de fusion :
    - Les nœuds présents dans git (par ID) écrasent ceux de Node-RED.
    - Les nœuds NOT présents dans git sont conservés (autres tabs non gérés par git).
    - Les nœuds dont le 'z' (tab parent) est géré par git sont retirés,
      sauf s'ils existent déjà dans git (ils seraient déjà inclus).
    """
    git_ids     = set(git_nodes_by_id.keys())
    git_tab_ids = {n["id"] for n in git_nodes_by_id.values() if n.get("type") == "tab"}

    # Garder les nœuds courants qui :
    #  - ne sont pas dans git (pas de remplacement prévu)
    #  - n'appartiennent pas à un tab géré par git (évite les doublons)
    kept = [
        n for n in current_flows
        if n.get("id") not in git_ids
        and n.get("z") not in git_tab_ids
    ]

    merged = kept + list(git_nodes_by_id.values())
    return merged, len(kept), len(git_nodes_by_id)


def wait_for_nodered(max_wait=30):
    """Attend que Node-RED soit disponible."""
    print(f"Attente de Node-RED ({NODERED_URL})...", end="", flush=True)
    for _ in range(max_wait):
        try:
            urllib.request.urlopen(f"{NODERED_URL}", timeout=2)
            print(" OK")
            return
        except Exception:
            print(".", end="", flush=True)
            time.sleep(1)
    print("\nErreur : Node-RED n'a pas répondu dans les délais.")
    sys.exit(1)


def main():
    print("=" * 60)
    print("Déploiement des flows Node-RED depuis git")
    print(f"  Source  : {os.path.abspath(FLOWS_DIR)}")
    print(f"  Cible   : {NODERED_URL}")
    print("=" * 60)

    wait_for_nodered()

    print("\nLecture des flows depuis git :")
    git_nodes = load_git_flows()
    print(f"  Total : {len(git_nodes)} nœuds uniques")

    print("\nRécupération des flows Node-RED actuels...")
    current = get_flows()
    print(f"  Flows actuels : {len(current)} nœuds")

    merged, kept, added = merge(current, git_nodes)
    print(f"\nFusion : {kept} nœuds conservés + {added} nœuds git = {len(merged)} total")

    print("\nDéploiement...")
    status = post_flows(merged)
    print(f"\n{'='*60}")
    print(f"✓ Flows déployés avec succès (HTTP {status})")
    print(f"  Ouvrir http://192.168.1.141:1880 pour vérifier")
    print("=" * 60)


if __name__ == "__main__":
    main()
