---
name: orchestration-maison-gabarits
description: "Tous les blocs à copier-coller verbatim dans un mandat ou un rapport, ordonnés par geste : frontmatter, Mission 0, sous-agents, footer de statut, bloc de lancement manuel + panneau ⚠️, pose pret-a-lancer, libération d'un mandat SUSPECT, réveil de l'orchestrateur, merge ff-only, preuve d'écran/checklist, garde de déploiement, pointeur de skill de compte, CLAUDE.md de dossier. Enfant de référence de SKILL.md — lu à CHAQUE pose de mandat ou traitement de rapport, jamais reconstruit de mémoire."
---

# Gabarits — orchestration-maison

Ordre : dans la séquence où un mandat les rencontre, de sa pose à sa clôture. Les `<...>` sont des
valeurs à remplacer ; celles en `<snake.case>` viennent de `config/harnais.json`.

Les gabarits de **documents** (mandat complet, rapport de doublage, passation, décision, leçon)
vivent dans `templates/gabarits/` — ce fichier-ci ne contient que les **gestes**.

## 1. Frontmatter de mandat

```
---
date: AAAA-MM-JJ
tags: [slug, thème]
session: Fxx
mandat: <slug-descriptif>
mandat_id: Mxxxx
statut: en-pose
modele: <valeur de modeles.autorises>
effort: <valeur de efforts_autorises>
worktree: <projet.racine>/<chemins.worktrees>/Fxx-Mxxxx
branche: <nom-de-branche>
derniere_maj: AAAA-MM-JJTHH:MM:SS+ZZ:ZZ
---
```

`mandat_id` = compteur `state.json.next_mandat_id` **relu à l'instant T**, jamais « le dernier vu
+ 1 » de mémoire. `derniere_maj` = sortie exacte de `TZ=<projet.timezone> date +%Y-%m-%dT%H:%M:%S%z`,
posée en dernier geste avant chaque écriture, jamais estimée. `statut` démarre à `en-pose`, bascule
à `pret-a-lancer` en tout dernier geste (§6) ou reste `prompt-disponible` pour un lancement
manuel (§5).

## 2. Mission 0 — scope git strict

```
1. git status --short
2. Si des fichiers modifies/nouveaux sous vault/reprise/, vault/decisions/, vault/revues/,
   vault/lecons/, ou <chemins.echanges>/archive/ (JAMAIS <chemins.echanges>/*.md hors archive/) :
   git add <ces chemins explicites uniquement>, commit
   (`docs(vault): reconciliation orchestrateur -- <raison>`), push, colle git ls-remote.
3. Si rien a committer dans ce scope : note-le, passe directement a la mission principale.
```

L'orchestrateur énumère à la pose les chemins EXACTS connus, plutôt que de recopier un `git add`
générique : ce gabarit est un filet de sécurité, pas un blanc-seing. Jamais `git add -A`/`-a`
(écrasement de merge vécu). Jamais un fichier d'échange hors `archive/` : il est la propriété
exclusive de la fenêtre qui le rédige. Jamais les registres de `<chemins.runtime>` : ils sont hors
git.

## 3. Sous-agents — les 4 règles

```
Un sous-agent `fork` herite du CONTEXTE ENTIER de la session (mandat complet, objectif final)
et des MEMES outils (ecriture, edition, git) -- une consigne etroite dans son prompt est une
instruction de portee, pas une restriction d'outils.
(1) lecture/recherche parallele = agents FRAIS (jamais `fork`) ;
(2) aucun sous-agent ne commit, ne pousse, ne modifie <chemins.runtime>/ ni vault/reprise/,
    ni ne tue un autre agent -- ces actes restent a la session mere ;
(3) tout rapport de sous-agent est VERIFIE (citations fichier:ligne recontrolees) avant
    d'entrer dans un livrable ;
(4) apres tout incident : git status/diff/log/reflog AVANT toute autre action.
```

## 4. Footer de statut — toute dernière ligne du rapport

```
Mxxxx
Implementation      ✅/❌
Tests               ✅/❌
Commit              ✅/❌
Branch push         ✅/❌
Review              ✅/❌
Merge main          ✅/❌
Main push           ✅/❌
TASK STATUS         <READY_FOR_INTEGRATION | IN_PROGRESS | BLOCKED | INTEGRATED>
PROJECT STATUS      <NOT_INTEGRATED | INTEGRATED>
```

`Mxxxx` = le `mandat_id` du frontmatter, jamais un numéro inventé. Chaque ligne = un fait
vérifiable par la session ELLE-MÊME à l'instant du rapport, jamais un objectif visé ni un « ça
devrait être bon ». `PROJECT STATUS: INTEGRATED` **uniquement** si `Merge main` ET `Main push` sont
✅ — sinon `NOT_INTEGRATED`, même si tout le reste est vert. Si le mandat touche un chemin de
`gardes_deploiement[]`, ajouter la ligne de garde correspondante (§11) : elle conditionne
`INTEGRATED` au même titre.

**Le tableau se vérifie, il ne se croit pas** (#28, #35) : l'orchestrateur, à chaque rapport traité, lance `node scripts/verifier-rendu.mjs effet --rapport <chemins.echanges>/Fxx.md` depuis la racine, après un `git fetch origin` ; le doubleur le relance, puis, pour un correctif, `node scripts/verifier-rendu.mjs rouge --branche <branche> --base <merge-base> --test '<commande de test>'`. Tout `ECART` se résout avant le doublage ou le GO.

## 5. Bloc de lancement manuel + panneau ⚠️

```
claude --model <modele> --effort <niveau> "Va lire ton prompt dans <chemins.echanges>/Fxx.md et exécute-le. Applique strictement les rituels décrits."
```

Guillemets DOUBLES DROITS `"` autour du trigger (jamais typographiques) ; la phrase du trigger ne
contient jamais de `"` ; une seule intervention du fondateur par lancement — jamais la commande sans
le trigger, jamais le trigger seul. **Réservé aux mandats à `statut: prompt-disponible`** : un
mandat `pret-a-lancer` ne reçoit JAMAIS ce bloc, le superviseur s'en charge (§6).

Panneau à coller OBLIGATOIREMENT au-dessus de tout bloc redonné une 2e fois :

```
⚠️ DÉJÀ TRANSMIS (1re fois : <contexte/heure>) — NE COLLE CE BLOC QUE SI AUCUNE FENÊTRE NE
TOURNE DÉJÀ SUR CE MANDAT. Deux sessions sur un même mandat = collision interdite.
```

Par défaut un bloc ne se redonne jamais (référence en une ligne). Relance d'un mandat possiblement
en cours : uniquement sur confirmation explicite du fondateur qu'aucune session ne tourne dessus,
jamais sur hypothèse. En cas de doute : traiter comme re-présenté → panneau.

## 6. Pose d'un mandat pour le superviseur

```
# 1. Écrire le fichier COMPLET avec statut: en-pose (frontmatter §1)
# 2. DERNIER GESTE, une seule ligne :
sed -i '' 's/^statut: en-pose$/statut: pret-a-lancer/' <chemins.echanges>/Fxx.md
# 3. Ne rien donner à coller au fondateur. Le superviseur lance au cycle suivant
#    (superviseur.poll_ms + superviseur.mtime_stable_ms).
```

Sous Linux, `sed -i` sans argument (`sed -i 's/.../.../'`). Un mandat que le fondateur doit lancer à
la main garde `statut: prompt-disponible` et reçoit le bloc §5.

## 7. Libérer un mandat SUSPECT — ordre impératif

```
1. <chemins.echanges>/Fxx.md : statut -> reponse-disponible (ou archiver + réécrire)   # D'ABORD
2. fermer l'espace de travail du mandat s'il est encore ouvert
3. rm <superviseur.home>/locks/Fxx.lock                                                 # ENSUITE
```

**L'ordre inverse relance le mandat au cycle suivant** : le superviseur scanne toutes les
`poll_ms`, et un fichier `pret-a-lancer` sans verrou est un candidat valide.

**Rien d'autre à faire, démon en marche** (M0026) : l'entrée correspondante de
`<superviseur.home>/etat-mandats.json` est oubliée au cycle suivant
(`ENTREE_ORPHELINE_OUBLIEE`, motif `verrou-absent`), **sans redémarrage**. Avant M0026 cette
entrée survivait au `rm` et volait les hooks du mandat suivant à reprendre le panneau —
incident réel du 23/09 : F04/M0021 fantôme, F01/M0024 privé de son `SessionStart` puis déclaré
SUSPECT à son tour.

## 8. Réveil de l'orchestrateur

Noms uniques et datés : `Réveil <projet.nom> AAAA-MM-JJ HH:MM` (réveil court) et
`Filet <projet.nom> AAAA-MM-JJ HH:MM` (filet) — date et heure de **tir** prévues, horloge
`projet.timezone`. Le message de chacun contient son nom exact, c'est par lui qu'on le retrouve pour
le supprimer, y compris parmi les tâches déjà tirées.

```
au tir, premier geste : supprimer le réveil qui vient de tirer (par son nom exact)
puis : lire les frontmatters des Fxx.md actifs et la fin de <chemins.runtime>/events.jsonl
puis : si reponse-disponible, traiter le rapport et poser le suivant en pret-a-lancer ;
       sinon, une ligne au fondateur
puis, seulement s'il faut continuer : réveil dans 15 min, nom "Réveil <projet.nom> AAAA-MM-JJ HH:MM"
(date et heure de tir, horloge projet.timezone), message :
"[RÉVEIL] Réveil <projet.nom> AAAA-MM-JJ HH:MM -- Vérifie les frontmatters des Fxx.md actifs et la fin de
<chemins.runtime>/events.jsonl. Si reponse-disponible : traite le rapport, puis pose le suivant en
pret-a-lancer. Sinon : une ligne au fondateur, et recrée un réveil (même règle de nom) si besoin de
continuer."
filet (règle de l'invariant ci-dessous) : dans 60 min, nom "Filet <projet.nom> AAAA-MM-JJ HH:MM",
message : "[FILET] Filet <projet.nom> AAAA-MM-JJ HH:MM -- Vérifie les frontmatters des Fxx.md actifs et
la fin de <chemins.runtime>/events.jsonl. Si reponse-disponible : traite le rapport, puis pose le
suivant en pret-a-lancer. Sinon : une ligne au fondateur, et recrée un réveil (même règle de nom) si
besoin de continuer."
```

**JAMAIS modifier un réveil existant** : la modification demande une approbation humaine et bloque
la boucle sans le fondateur. On supprime et on reprogramme. Invariant : au plus un réveil court + un
filet vivants ; filet à `+60 min` si le réveil court échoue, supprimé et recréé (jamais deux filets)
quand le réveil court tire et qu'on en recrée un.

Fin de tâche : zéro réveil de cette fenêtre d'orchestration dans la liste des tâches planifiées — le
vérifier et le dire au fondateur en une ligne. Périmètre : on ne supprime que ses propres réveils,
jamais une tâche planifiée récurrente ni celle d'une autre fenêtre ou d'un autre projet, sans accord
explicite du fondateur.

## 9. Merge vers la branche principale — scratch puis ff-only

```
R=<projet.racine> ; P=<git.branche_principale> ; S=$R/<chemins.worktrees>/<Fxx>-merge   # <tip> = tip de la branche doublée
git -C $R fetch origin && git -C $R worktree add --detach $S origin/$P
git -C $S merge --no-ff <tip> -m "Merge <branche> [<Rxxx>]" -m "<git.trailer>"   # tests SUR ce résultat
cp -- $R/vault/revues/<AAAA-MM-JJ-Rxxx-slug>.md $S/vault/revues/   # le rapport rendu, copié explicitement dans le scratch
git -C $S add -- vault/revues/<AAAA-MM-JJ-Rxxx-slug>.md && git -C $S commit -m "docs(revues): <Rxxx> -- GO" -m "<git.trailer>"   # frontmatter : verdict: GO + tip: <sha 40 de <tip>>
git -C $R merge --ff-only $(git -C $S rev-parse HEAD)     # depuis la racine ; refus = rapport
git -C $R push origin $P && git ls-remote origin refs/heads/$P    # ls-remote collé au rapport
git -C $R worktree remove $S      # puis idem pour le worktree de doublage
```

Le rapport de doublage GO est committé dans le scratch **après** le `merge --no-ff` et **avant** le
`ff-only`, pour partir dans le **même push** que le merge : le hook `pre-push` refuse tout push sur
`<principale>` qui introduit un merge sans rapport GO pour le tip mergé dans l'arbre poussé.
Un rapport de doublage couvre une seule branche ; son frontmatter, fermé par `---`, porte
**exactement une** ligne `verdict: GO` **et exactement une** ligne `tip: <sha 40>` (SHA complet du tip
de cette branche, `git rev-parse <tip>`) ; une citation du tip dans le corps ne compte pas.
Commits directs sur la principale : seulement des chemins `vault/` ; tout le reste passe par une
branche, un doublage et un merge (décision Malik 2026-09-24, #41).

Jamais `stash`/`reset --hard`/`checkout -- .`/`update-ref`/`pull --rebase --autostash` à la racine
partagée. Refus `ff-only` = rapport, jamais de force ; si `origin/<principale>` a bougé entre le
scratch et le `ff-only`, refaire le scratch UNE fois. Le `ff-only` met le working tree à jour : plus
de ligne `D` fantôme. Les fichiers modifiés localement à la racine (carnet, `reprise/`, `Fxx.md` en
pose) bloquent le `ff-only` s'ils sont touchés par le merge → les committer d'abord, en pathspec
explicite (`git commit -- <chemins>`), jamais `-a`. **PAS** les registres de `<chemins.runtime>` :
gitignorés, jamais committés.

## 10. Preuve d'écran / checklist fondateur

Preuve d'écran : captures **RÉELLES** de la route réelle dans un client réel, **avant ET après**,
enregistrées sous `vault/revues/captures-<Mxxxx>/`, nommées de façon parlante, citées par leur nom
de fichier dans le rapport à côté de ce qu'elles démontrent. Un harnais local, un composant monté en
isolation, une lecture de code ou un `git diff` **ne sont pas** des preuves d'écran — ils peuvent
l'étayer, jamais la remplacer. Captures à md5 distincts et à noms parlants (même taille en octets =
suspect ; md5 distincts ≠ états distincts, il faut LIRE l'image). Le doublage refait les captures
lui-même. Jamais deux mandats à preuve d'écran en parallèle si l'outil de capture est unique.

Capture impossible → **deux obligations cumulatives** :

```
1. L'axe UX du verdict ne peut PAS être GO : ⚠️ « non vérifié à l'écran », dit tel quel.
2. CHECKLIST DE TEST POUR LE FONDATEUR : 3 à 5 gestes concrets et ordonnés — URL exacte,
   état de départ, action, ET CE QU'IL DOIT VOIR. Pas « vérifier que l'affichage est correct »,
   mais « ouvrir <route>, cliquer <bouton>, le bouton doit devenir gris pendant l'opération ».
   Transmise telle quelle par l'orchestrateur dans sa réponse au fondateur.
```

## 11. Garde de déploiement

Pour chaque entrée de `gardes_deploiement[]` dont un `chemins[]` est touché par le merge :

```
<garde.preuve>          # ex. : ssh <hôte> "cd <racine> && git rev-parse --short HEAD"
```

Comparer la sortie au SHA mergé, **preuve collée au rapport** — obligatoire avant de déclarer
`PROJECT STATUS: INTEGRATED`. Le périmètre d'une garde s'étend explicitement le jour où un nouveau
service long-lived apparaît, jamais supposé exhaustif par défaut.

Cible injoignable → **rien de forcé** : écrire le SHA en attente dans `state.json.projet` sous la
clé de la garde, `PROJECT STATUS` reste `NOT_INTEGRATED`, à reprendre dès que la cible répond.

## 12. Skill de compte — pointeur (texte exact)

```
---
name: "orchestration-<projet>"
description: "Pointeur vers la convention d'orchestration du dépôt <projet> (source de vérité unique : <racine>/.claude/skills/orchestration-bureau/). Charger en tout début de session d'orchestration sur <projet> ; sans le dossier accessible, STOP."
---

# orchestration-<projet> — POINTEUR

Une seule source de vérité : le dépôt. Ce skill de compte ne duplique pas la convention — une copie
a toujours une version de retard.

## Ce que tu fais, dans l'ordre

1. Vérifie que le dossier du projet est accessible (racine `<racine>`). **Non accessible → STOP** :
   demande au fondateur de connecter le dossier, n'orchestre rien de mémoire.
2. Lis EN ENTIER `.claude/skills/orchestration-bureau/SKILL.md` du dépôt — c'est la convention qui
   fait foi, à sa version courante.
3. Lis `<racine>/config/harnais.json` : tous les paramètres du projet y sont.
4. Lis À LA DEMANDE, aux points où le parent te le dit : `references/gabarits.md` (à CHAQUE pose de
   mandat ou traitement de rapport).
5. Applique ensuite §1 du parent (reprise de session).

## Règles de ce pointeur

- Aucune règle d'orchestration ne vit ici. Si ce fichier et le dépôt divergent, **le dépôt gagne** ;
  signale l'écart au fondateur.
- Ce fichier n'est mis à jour que si le CHEMIN de la source change. Une nouvelle version de la
  convention se grave dans le dépôt seul, par mandat doublé — jamais ici.
```

## 13. `CLAUDE.md` de dossier et rules à `paths:` — consigne, question, exemple

Règle : SKILL.md §12. Consigne à coller dans un mandat dont le diff touche un dossier doté d'un
`CLAUDE.md` ou couvert par une rule à `paths:` :

```
Les CLAUDE.md imbriques et les rules a `paths:` (.claude/rules/*.md) font foi au meme titre que
le CLAUDE.md racine. Si ton diff change un comportement decrit dans l'un d'eux, mets-le a jour
DANS LE MEME COMMIT. Un CLAUDE.md de dossier ne contient que les commandes et les pieges
d'environnement du dossier : doctrine -> rule a `paths:` ; historique et compteurs dates ->
vault/ ; ce qui coute de l'argent ou touche une ressource irreversible -> CLAUDE.md racine.
```

Question du doublage (verbatim, un axe du rapport R0xx) :

```
Le diff touche-t-il un comportement decrit dans un CLAUDE.md imbrique ou une rule a `paths:` ?
Si oui, sont-ils a jour dans le meme commit ? Un CLAUDE.md de dossier contient-il autre chose
que des commandes et des pieges ?
```

Exemple de `CLAUDE.md` de dossier conforme — deux sections, « Commandes » et « Pièges », rien d'autre :

```markdown
# <dossier>/ -- commandes et pieges

## Commandes
- Tests : `make test` (lance depuis ce dossier, pas depuis la racine)
- Serveur local : `make serve PORT=8081`

## Pieges
- Le port par defaut est pris par un autre service de la machine : toujours `PORT=8081`.
- `OFFLINE=1` avant `make test` sans reseau, sinon les tests d'integration attendent leur timeout.
```
