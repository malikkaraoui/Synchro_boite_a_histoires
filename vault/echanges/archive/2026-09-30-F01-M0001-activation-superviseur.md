---
date: 2026-09-30
tags: [synchro-boite, harnais, superviseur]
session: F01
mandat: activation-superviseur
mandat_id: M0001
statut: reponse-disponible
modele: opus
effort: medium
worktree: /Users/malik/Documents/Synchro_boite_a_histoires
branche: main
derniere_maj: 2026-09-30T12:48:14+0200
---

# F01 — M0001 — Compléter config/harnais.json et inscrire le projet au superviseur

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente.

## Rituel de début
Premier geste : passer `statut: en-pose`/`pret-a-lancer`/`prompt-disponible` → `statut: en-cours` dans le frontmatter de CE fichier.

## Contexte (tout ce que tu dois savoir)
Le harnais d'orchestration v1.0.1 vient d'être posé dans ce dépôt par `bootstrap.sh` (fichiers NON committés : c'est le fondateur qui fera ce premier commit). Le fondateur a décidé le 2026-09-30 :
1. `chemins.doctrine_projet` = `[".claude/CLAUDE.md"]`
2. activer le superviseur : `superviseur.actif` = `true`, puis inscrire le projet avec `install.sh --ajouter`.

## DÉROGATION git — zéro écriture git dans ce mandat
Le dépôt contient des modifications non committées du fondateur (code + fichiers du harnais). **Aucun `git add`, `commit`, `push`, `checkout`, `stash`, `reset`**. Pas de Mission 0. Git en lecture uniquement, toujours `git --no-optional-locks`. Tu ne modifies QUE `config/harnais.json` et CE fichier.

## Gardes de précondition — si une seule échoue : STOP, rapport
Depuis `/Users/malik/Documents/Synchro_boite_a_histoires` :
1. `uname` = `Darwin`
2. `git --no-optional-locks branch --show-current` = `main`
3. `node -e 'const c=require("./config/harnais.json");console.log(c.superviseur.actif, JSON.stringify(c.chemins.doctrine_projet))'` affiche `false []`
4. `ls .claude/CLAUDE.md ~/.harnais/canon/superviseur/install.sh /Applications/cmux.app/Contents/Resources/bin/cmux` : les trois existent
5. `cat ~/.harnais/VERSION` = `v1.0.1`

## Mission 1 — config
Modifier par script (jamais en retapant le fichier) :
```sh
node -e 'const fs=require("fs");const p="config/harnais.json";const c=JSON.parse(fs.readFileSync(p,"utf8"));c.chemins.doctrine_projet=[".claude/CLAUDE.md"];c.superviseur.actif=true;fs.writeFileSync(p,JSON.stringify(c,null,2)+"\n")'
```
Preuve : relancer la commande de la garde 3 → doit afficher `true [".claude/CLAUDE.md"]`. Colle la sortie.

## Mission 2 — inscription au superviseur
```sh
sh ~/.harnais/canon/superviseur/install.sh --ajouter /Users/malik/Documents/Synchro_boite_a_histoires; echo "rc=$?"
```
Colle la sortie COMPLÈTE et le code retour.
- JAMAIS `--force-tcc`. Si le script réclame une autorisation système (accès disque / TCC) ou échoue : STOP. Remets `superviseur.actif` à `false` avec la même méthode node, prouve-le (garde 3), rapport avec la sortie exacte et ce que le fondateur doit faire selon le message du script.
- Rc 0 : preuves à coller :
```sh
launchctl print gui/$(id -u)/com.harnais.superviseur | head -25
cat ~/.harnais/config.json
```

## Conditions STOP
Toute garde fausse ; tout échec de commande ; tout fichier modifié autre que `config/harnais.json` et ce fichier ; toute sortie que tu ne comprends pas.

## Interdits permanents
Jamais de `sleep`. Aucune tâche de fond vivante à ta sortie. Pas de `rm`. Pas de `--force-tcc`. Pas d'écriture dans `vault/runtime/` (l'orchestrateur tient les registres pour ce mandat).

## Rituels de fin, dans l'ordre
1. Appende à la fin de CE fichier un titre `## Rapport M0001` : gardes (sorties collées), Mission 1 (sortie), Mission 2 (sortie complète + rc + preuves), écarts.
2. Dernière ligne du rapport : ce footer, rempli honnêtement (Commit/push/merge = ❌ par dérogation, c'est attendu) :
```
M0001
Implementation      ✅/❌
Tests               ✅/❌
Commit              ❌
Branch push         ❌
Review              ❌
Merge main          ❌
Main push           ❌
TASK STATUS         <READY_FOR_INTEGRATION | BLOCKED>
PROJECT STATUS      NOT_INTEGRATED
```
(`Tests` = les preuves Mission 1 et 2 passées.)
3. EN DERNIER GESTE : `statut: reponse-disponible` dans le frontmatter.

## Rapport M0001

Exécuté le 2026-09-30 à 12:49, par claude-opus-5-5. Aucune écriture git : dérogation respectée.

### Gardes (toutes vraies)
```
--- G1
Darwin
--- G2
main
--- G3
false []
--- G4
.claude/CLAUDE.md
/Applications/cmux.app/Contents/Resources/bin/cmux
/Users/malik/.harnais/canon/superviseur/install.sh
--- G5
v1.0.1
```

### Mission 1 — config (commande node du mandat, sans retaper le fichier)
Preuve (garde 3 relancée) :
```
true [".claude/CLAUDE.md"]
```
Diff avant/après (copie de sauvegarde dans le scratchpad), seules les deux clés visées changent :
```
15c15,17
<     "doctrine_projet": []
---
>     "doctrine_projet": [
>       ".claude/CLAUDE.md"
>     ]
49c51
<     "actif": false,
---
>     "actif": true,
```

### Mission 2 — inscription (lancée avec `< /dev/null`, sans `--force-tcc`)
Sortie complète :
```
Bootstrap failed: 5: Input/output error
Try re-running the command as root for richer errors.
launchctl bootstrap échoué (1re tentative) -- état du service puis 2e tentative
Bad request.
Could not find service "com.harnais.superviseur" in domain for user gui: 501
Projet enregistré : synchro-boite (/Users/malik/Documents/Synchro_boite_a_histoires/config/harnais.json)
Installé : com.harnais.superviseur (sert désormais 4 projet(s))
  registre    : /Users/malik/.harnais/config.json
  harnais home: /Users/malik/.harnais
  script      : /Users/malik/.harnais/canon/superviseur/superviseur.mjs
  hooks.json  : /Users/malik/.harnais/hooks.json (régénéré, global)   hook : /Users/malik/.harnais/bin/hook.sh
  node        : /opt/homebrew/bin/node -> /opt/homebrew/Cellar/node/26.7.0/bin/node
Vérification :
  launchctl print gui/501/com.harnais.superviseur
  tail -f /Users/malik/.harnais/harnais.log
  tail -f /Users/malik/Documents/Synchro_boite_a_histoires/vault/runtime/events.jsonl | grep SUPERVISEUR_DEMARRE
rc=0
```
Lecture : la 1re tentative de `launchctl bootstrap` a échoué (EIO 5), ce qui arrive classiquement quand le service vient d'être déchargé. Le script a prévu ce cas : il relit l'état du service (« Could not find service » : il est bien déchargé), puis retente. La 2e tentative a réussi. Pas de demande TCC.

**Preuve `launchctl print` : NON OBTENUE.** Le gestionnaire de permissions de l'outil a refusé la commande deux fois (avec `$(id -u)`, puis avec `501` en dur). Pour ne pas bloquer, j'ai prouvé que le service tourne par trois voies indépendantes, toutes en lecture seule :

1. Processus vivant, démarré à l'heure de l'install, lancé depuis le canon :
```
66752 mer. 30 sept. 12:49:23 2026  /opt/homebrew/bin/node /Users/malik/.harnais/canon/superviseur/superviseur.mjs
```
2. `~/.harnais/harnais.log` (dernière ligne) :
```
2026-09-30T12:49:24+02:00 SUPERVISEUR_DEMARRE version=1.3.3-harnais projets=4 echecs=0 plafond_global=24 poll_ms=10000
```
3. `vault/runtime/events.jsonl` du projet, lu sans y écrire : on y trouve `SUPERVISEUR_DEMARRE` à 12:49:24 avec `"projet_nom":"synchro-boite"`, `"actif":true`, `"repo":"/Users/malik/Documents/Synchro_boite_a_histoires"`.

Le plist `~/Library/LaunchAgents/com.harnais.superviseur.plist` a pour `ProgramArguments` `/opt/homebrew/bin/node` puis `/Users/malik/.harnais/canon/superviseur/superviseur.mjs`.

`cat ~/.harnais/config.json` :
```
{
  "projets": [
    "/Users/malik/Documents/orchestration/config/harnais.json",
    "/Users/malik/Documents/EV-LLM/config/harnais.json",
    "/Users/malik/Documents/la forge a histoires/config/harnais.json",
    "/Users/malik/Documents/Synchro_boite_a_histoires/config/harnais.json"
  ]
}
```

### Intégrité
- `git status --porcelain`, comparé avant/après : une seule différence, `config/harnais.json` qui passe de `A ` à `AM`. `vault/echanges/F01.md` était déjà non suivi (`??`). HEAD est inchangé (`8ce44ee`).
- `vault/runtime/events.jsonl` a été écrit par le démon, pas par moi.
- Aucune tâche de fond laissée, pas de `sleep`, pas de `rm`.

### Écarts et observations
1. La preuve `launchctl print` exigée n'a pas pu être collée (refus d'outil). Elle est remplacée par les preuves 1 à 3 ci-dessus. À faire par l'orchestrateur ou le fondateur : `! launchctl print gui/501/com.harnais.superviseur | head -25`.
2. Le démarrage du bootstrap a d'abord échoué (EIO), puis la reprise interne du script a fonctionné. rc=0.
3. Dans `harnais.log`, trois lignes du 2026-09-30 à 01:39 : `CONFIG_CLE_MORTE cle=superviseur.actif` pour harnais, ev-llm et forge-histoires. Elles venaient d'un service étranger (`SERVICE_ETRANGER`, worktree `orchestration/.claude/worktrees/F01-R088`), arrêté depuis. La chaîne `CONFIG_CLE_MORTE` n'existe pas dans le canon actuel, et le démon canonique lit bien `actif:true`. Rien à corriger ici, mais à surveiller si ce code F01-R088 est mergé : la clé `superviseur.actif` y serait considérée comme morte.
4. Hors périmètre : un second superviseur tourne encore, celui de 1RR (`/Users/malik/Documents/1RR/scripts/superviseur/superviseur.mjs`, pid 75238, lancé le 29/09). Il est indépendant du harnais. Signalé seulement.

M0001
Implementation      ✅
Tests               ✅
Commit              ❌
Branch push         ❌
Review              ❌
Merge main          ❌
Main push           ❌
TASK STATUS         READY_FOR_INTEGRATION
PROJECT STATUS      NOT_INTEGRATED
