---

📘 Guide complet : GLM-5 en local sur Windows (sans compte chinois)

✅ Pourquoi cette méthode ?

Avantage Explication
Pas d'inscription Rien à créer, pas de compte bigmodel.cn
Totalement gratuit Pas d'API payante, pas d'abonnement
100% privé Tout tourne sur ta machine, aucune donnée envoyée
Fonctionne hors ligne Une fois téléchargé, pas besoin d'internet
Interface en anglais/CLI Pas de chinois à traduire

⚠️ Prérequis matériel

Avant de commencer, vérifie ta configuration :

Composant Minimum Recommandé pour GLM-5
RAM 32 Go 64 Go+
Stockage 30 Go 100 Go+ (SSD)
GPU (optionnel) - NVIDIA avec 8-12 Go VRAM accélère
OS Windows 10/11 Windows 10/11

⚠️ Important : GLM-5 complet fait 241 Go en version compressée. Si tu as 32 Go de RAM, je te recommande plutôt GLM-4.7-Flash (beaucoup plus léger) ou GLM-5-Turbo. Je te donne les deux options.

---

🚀 Méthode 1 : La plus simple (Ollama + modèle léger)

Ollama est l'outil le plus simple pour faire tourner des modèles localement, sans configuration complexe .

Étape 1 : Installer Ollama

1. Va sur ollama.com/download
2. Télécharge OllamaSetup.exe pour Windows
3. Double-clique et installe comme n'importe quel logiciel
4. Ollama se lance automatiquement en arrière-plan (icône dans la barre des tâches)

Étape 2 : Télécharger et lancer GLM-5

Ouvre PowerShell ou Invite de commandes (cmd) et tape :

```cmd
ollama pull glm-5:cloud
```

⚠️ Ce modèle fait environ 30 Go. Assure-toi d'avoir assez d'espace disque et une bonne connexion.

Variantes plus légères si 30 Go c'est trop :

Modèle Taille Commande
GLM-4.7-Flash (recommandé pour 32 Go RAM) ~20 Go ollama pull glm-4.7-flash 
GLM-5-Turbo ~25 Go ollama pull glm-5-turbo

Étape 3 : Vérifier que ça fonctionne

```cmd
ollama run glm-5:cloud
```

Tu devrais voir un prompt >>>. Tape un message :

```
>>> Qui es-tu ?
```

Si le modèle répond, tout fonctionne ! Tape /bye pour quitter.

Étape 4 : Utiliser GLM-5 avec Claude Code

Maintenant que GLM-5 tourne localement, on va connecter Claude Code à ce modèle local.

Installer Claude Code :

```cmd
npm install -g @anthropic-ai/claude-code
```

Configurer Claude Code pour utiliser Ollama :

Crée (ou modifie) le fichier de configuration :

· Chemin : C:\Users\TON_NOM_UTILISATEUR\.claude\settings.json

```json
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "ollama",
    "ANTHROPIC_BASE_URL": "http://localhost:11434/v1",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "glm-5:cloud",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "glm-5:cloud",
    "API_TIMEOUT_MS": "3000000",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": 1
  }
}
```

Lance Claude Code :

```cmd
claude
```

Tu utilises maintenant Claude Code avec GLM-5 en local, sans aucun compte, sans API payante .

---

🖥️ Méthode 2 : Version complète (241 Go) avec llama.cpp

Si tu as 64 Go de RAM ou plus et veux la puissance maximale du GLM-5 complet (744B paramètres), voici la méthode .

Étape 1 : Installer les dépendances

```cmd
# Installer Python (si pas déjà fait)
# Va sur python.org, télécharge Python 3.10+

# Installer git (si pas déjà fait)
# Va sur git-scm.com/download/win
```

Étape 2 : Compiler llama.cpp avec support GLM-5

```cmd
git clone https://github.com/ggml-org/llama.cpp
cd llama.cpp
git fetch origin pull/19460/head:MASTER
git checkout MASTER
mkdir build && cd build
cmake .. -DGGML_CUDA=OFF  # Désactive CUDA si pas de GPU NVIDIA
cmake --build . --config Release -j
cd ..
```

Étape 3 : Télécharger le modèle quantifié

```cmd
pip install -U huggingface_hub hf_transfer
set HF_HUB_ENABLE_HF_TRANSFER=1
hf download unsloth/GLM-5-GGUF --local-dir GLM-5-GGUF --include "*UD-IQ2_XXS*"
```

⏱️ Cette étape peut prendre plusieurs heures (241 Go à télécharger).

Étape 4 : Lancer le serveur

```cmd
.\llama-server.exe --model GLM-5-GGUF\UD-IQ2_XXS\GLM-5-UD-IQ2_XXS-00001-of-00006.gguf --alias "glm-5" --ctx-size 32768 --port 8000
```

Ton modèle est maintenant accessible sur http://localhost:8000/v1 .

---

🔧 Méthode 3 : Alternative légère (32 Go RAM)

Avec ta configuration (32 Go RAM), je te recommande cette approche :

Option A : GLM-4.7-Flash (recommandé)

C'est le modèle qui a des performances équivalentes à Claude Sonnet 4.5 .

```cmd
ollama pull glm-4.7-flash
ollama run glm-4.7-flash
```

Option B : Qwen2.5-Coder 32B (encore plus léger)

Si tu veux un modèle spécialisé code qui tient facilement :

```cmd
ollama pull qwen2.5-coder:32b
ollama run qwen2.5-coder:32b
```

---

🎯 Pour ton workflow actuel (Claude Desktop)

Si tu veux absolument garder l'interface Claude Desktop mais utiliser GLM-5 localement, c'est plus compliqué. Claude Desktop ne supporte que l'API d'Anthropic.

La meilleure alternative : utilise Claude Code en ligne de commande (méthode 1 étape 4) avec GLM-5 en local, puis fais tes commits manuellement comme avant.

Ou utilise Open WebUI : une interface web locale qui se connecte à ton modèle Ollama  :

```cmd
# Installer Docker Desktop
# Puis lancer Open WebUI
docker run -d -p 3000:8080 --add-host=host.docker.internal:host-gateway -v open-webui:/app/backend/data --name open-webui --restart always ghcr.io/open-webui/open-webui:main
```

Puis va sur http://localhost:3000 – tu auras une interface web avec GLM-5.

---

📊 Récapitulatif des options

Méthode Téléchargement RAM requise Inscription Interface
Ollama + GLM-4.7-Flash ~20 Go 32 Go ❌ Aucune CLI ou Open WebUI
Ollama + GLM-5:cloud ~30 Go 48 Go+ ❌ Aucune CLI ou Open WebUI
llama.cpp + GLM-5 complet 241 Go 64 Go+ ❌ Aucune Serveur local
Claude Code + Ollama 20-30 Go 32 Go+ ❌ Aucune CLI (avec GitHub)

---

✅ Mon conseil pour toi

Avec 32 Go de RAM et ton besoin de développement JS/Rust/React :

1. Installe Ollama (5 minutes)
2. Télécharge GLM-4.7-Flash : ollama pull glm-4.7-flash
3. Teste : ollama run glm-4.7-flash
4. Installe Claude Code et configure-le pour utiliser Ollama (fichier settings.json)
5. Travaille normalement – tu fais tes commits toi-même comme avant
