---
date: AAAA-MM-JJ
tags: [git, hooks, harnais]
---

# Hooks git du harnais

Trois hooks, tous en `sh` POSIX, sans dépendance :

| Hook | Ce qu'il fait | Bloquant ? |
|---|---|---|
| `pre-commit` | Refuse un commit qui ferait dépasser la limite d'octets à l'index de reprise, et **imprime le remède** dans le message d'erreur. | Oui (c'est son objet) |
| `commit-msg` | Impose le trailer de signature du projet, quelle que soit la signature proposée par l'outil appelant. | **Jamais** (`exit 0` en toute circonstance) |
| `pre-push` | Sur la branche principale, ne laisse passer que les merges couverts par un rapport de doublage GO (`tip:` = le tip mergé) et les commits qui ne touchent que `vault/`. | Oui, **sur la branche principale seulement** |

## Activation

```
git config core.hooksPath <chemin de ce dossier dans le projet>
```

Par exemple, si le scaffold est copié dans `scripts/git-hooks/` :

```
git config core.hooksPath scripts/git-hooks
chmod +x scripts/git-hooks/pre-commit scripts/git-hooks/commit-msg scripts/git-hooks/pre-push
```

Vérifier : `git config --get core.hooksPath`. Le bit exécutable est nécessaire — un hook non
exécutable est ignoré **en silence**, et le garde-fou disparaît sans que rien ne le signale.

## Pourquoi hors de `.git/hooks`

`.git/hooks/` n'est **pas versionné** et n'est pas copié par un `git clone`. Un hook qui y vit
n'existe que sur la machine où quelqu'un l'a installé à la main : il protège un poste, pas le
projet. Déportés dans un dossier versionné pointé par `core.hooksPath`, les hooks sont revus
comme du code, voyagent avec le dépôt, et leur modification laisse une trace.

Contrepartie assumée : `core.hooksPath` est une configuration **locale**, à poser une fois par
clone (et par worktree qui ne partage pas la config). L'installation reste un geste explicite ;
c'est le contenu qui est partagé, pas l'activation.

## Paramétrage — rien n'est codé en dur

### `pre-commit`

| Variable | Défaut | Rôle |
|---|---|---|
| `ROTATION_INDEX` | `vault/reprise/00_INDEX.md` | Le fichier surveillé. |
| `ROTATION_LIMITE` | `80000` | La limite, en octets. |
| `ROTATION_REMEDE` | texte à remplacer | La commande de rotation imprimée dans l'erreur. |

La taille mesurée est celle de la version **indexée** (`git cat-file -s ":$INDEX"`), pas celle du
disque : c'est le contenu qui sera réellement committé.

### `commit-msg`

La valeur du trailer n'est jamais dans le hook. Elle est cherchée, dans l'ordre :

1. `HARNAIS_TRAILER` (variable d'environnement) ;
2. le fichier de configuration du projet — `HARNAIS_CONFIG`, défaut `config/harnais.json`, clé
   `"trailer"`.

**Si aucune valeur n'est trouvée, le hook ne touche à rien et sort 0** : dégradation contrôlée,
jamais un commit perdu parce qu'une configuration manquait.

**Retrait des trailers concurrents — décision du 2026-09-23 (Malik, « Retrait automatique »,
#29).** Avant d'ajouter le trailer du projet, le hook retire les lignes qui matchent un motif
étendu (ERE, évalué par `awk`). Le motif est cherché, dans l'ordre :

1. `TRAILERS_RETIRES` (variable d'environnement) — même vide : vide = ne rien retirer ;
2. la clé `"trailers_retires"` du fichier de configuration (bloc `git`) — même vide : vide = ne
   rien retirer (opt-out explicite) ;
3. à défaut de clé : `^[Cc][Oo]-[Aa][Uu][Tt][Hh][Oo][Rr][Ee][Dd]-[Bb][Yy]:`.

Défaut (config livrée et repli) : **toute ligne `Co-Authored-By:` est retirée, sans casse** — Git
traite les clés de trailer sans casse, donc `Co-authored-by:` est retiré comme `Co-Authored-By:`.
Seules les lignes qui *commencent* par la clé sont visées : une mention dans le corps est intacte.
**La ligne exacte du trailer projet n'est jamais retirée**, même si elle matche le motif.

Fail-safe : config illisible ou motif invalide → rien n'est retiré, le trailer projet est quand
même ajouté, une ligne est écrite sur stderr, et le hook sort 0. Le hook ne refuse jamais un commit.

Règle révoquée (tracée, R4) : jusqu'au 2026-09-23, `TRAILERS_RETIRES` était vide par défaut et rien
n'était retiré (« geste destructeur, jamais appliqué par défaut »). Le commit `d7cefef` a montré
le coût : un trailer d'outil survivait à côté du trailer projet.

Témoins : `node --test scripts/harnais-hooks/commit-msg.test.mjs` (`HOOK_COMMIT_MSG=<chemin>` pour
rejouer la suite sur une autre version du hook).

Le hook est idempotent : relancé sur un message déjà signé, il ne duplique pas le trailer.

### `pre-push`

Décision du 2026-09-24 (Malik, #27 recadrée : « simple a efficace / on essai ») : la règle « rien ne
merge sans doublage indépendant » (convention §9) ne tient plus seulement par la discipline.

Entrée (`githooks(5)`, section pre-push) : une ligne par ref poussée,
`<local-ref> SP <local-object-name> SP <remote-ref> SP <remote-object-name> LF` ; ref absente du
distant = `<remote-object-name>` tout à zéro ; suppression = `<local-ref>` `(delete)` et
`<local-object-name>` tout à zéro.

**La règle, en une phrase (décision Malik du 2026-09-24 00:55, #41)** : sur `main`, seuls passent
les merges avec rapport GO et les commits qui ne touchent que `vault/`. « Tout le reste est refusé,
et il faut passer par une branche, un doublage et un merge. »

Branche principale : clé `"branche_principale"` du bloc `"git"` de `HARNAIS_CONFIG` (défaut
`config/harnais.json`). Seul ce bloc compte : une clé de même nom dans un autre bloc est ignorée
(O4, témoin P28). Config absente, clé absente ou JSON illisible par le mini-lecteur : `main`.
Config présente mais non lisible (droits) : erreur interne, refus sur `main`.

Pour chaque ligne visant `refs/heads/<branche principale>` :

1. **plage introduite** : `remote..local` ; distant nul ou inconnu localement : `local --not --remotes` ;
   suppression : laissée passer ;
2. **commits directs (O3)** : chaque commit **non-merge** de la chaîne `git rev-list --first-parent`
   de la plage ne doit toucher que des chemins sous `vault/` (journaux, archives, rapports). Les
   chemins sont lus par `git diff-tree --no-renames` : un renommage `scripts/x` → `vault/x` compte
   comme un chemin hors `vault/` (témoin P22). Un fast-forward de `main` vers le tip d'une branche
   est donc refusé : les commits de la branche deviennent premier parent (P18). Les commits amenés
   par le **second** parent d'un merge ne sont pas contrôlés un par un : le rapport GO du merge les
   couvre ;
3. **merges** : pour chaque commit de merge de la plage, `T` = son **second parent** (le tip mergé) :
   il faut, dans l'**arbre poussé** (`git ls-tree` sur `local`, jamais le working tree ni l'index),
   un fichier `vault/revues/*.md` dont le frontmatter
   - commence par `---` en première ligne et se **ferme** par une ligne `---` (O2) ;
   - porte **exactement une** ligne `verdict:`, égale à `GO` (O2) ;
   - porte **exactement une** ligne `tip:`, égale au **SHA complet** (40 caractères) de `T` (O1).

   Le corps du rapport ne compte plus : un SHA cité en passant ne couvre rien (P24), un SHA court non
   plus (P25). Un rapport malformé ou contradictoire est ignoré, jamais une erreur (P26, P27).
   **Exception** (option B, décision Malik du 2026-09-24 00:28, « go pour b ») : si `T` est **déjà
   publié** — ancêtre de `remote` quand il est non nul, sinon atteignable depuis une ref
   `refs/remotes/*` — le merge est ignoré : il n'introduit aucun travail non doublé. C'est le cas
   d'un merge de la principale **dans** une branche (base périmée, convention §8 ; témoins P13, P16).
   Un tip poussé sur une *autre* branche distante n'est **pas** « publié » au sens de cette règle
   quand `remote` est connu : le merge final d'une branche doublée reste exigé (témoin P17) ;
4. sinon refus (`exit 1`) : le commit fautif et **un** chemin hors `vault/`, ou le merge et le SHA
   de `T` qui manque, puis la marche à suivre.

Forme attendue d'un rapport de doublage (gabarit §9) :

```
---
verdict: GO
tip: <SHA complet, 40 caractères, du tip doublé>
---
```

**Un rapport de doublage = une branche** : un doublage multi-branches à verdict global `RESERVE`, ou
une réserve levée par un mandat correctif sans re-doublage `GO`, ne couvre rien.

Rétro-test (historique, rien à réécrire) :
- M0056/M0057, règle d'alors (« le texte cite les 7 premiers caractères de `T` ») sur les six merges
  de l'arbre `a5c36e5` : 4 acceptés, **`1fc4f53` et `5473359` refusés a posteriori** —
  doublage multi-branches (R010) / réserve levée sans re-doublage (R009, M0029) ;
- M0058, règle `tip:` : les rapports antérieurs à R020 ne portent pas `tip:`. Même invocation
  (plage `21df1f5..a5c36e5`) : les **six** merges sont refusés a posteriori, aucun commit direct
  refusé (ce sont des rapports sous `vault/`). Plage réelle `a5c36e5..786da57` : **acceptée** — le
  merge `75428f8` est couvert par R020 (`verdict: GO`, `tip: 8a9b5840…`), committé en `786da57` et
  poussé avec lui ; poussé seul, `75428f8` serait refusé. Rituel d'alors, historique non réécrit ;
  le prochain merge fait selon le nouveau §9 passe (témoin P23).

**Conséquence sur le rituel de merge** : le rapport de doublage GO est committé dans le scratch,
**après** le `merge --no-ff` et **avant** le `ff-only`, pour partir dans le même push que le merge
(gabarit §9).

Fail-closed sur la règle ; erreur interne du hook (git absent, config illisible, commande git en
échec) : refus **sur la branche principale seulement**, un push de branche de travail n'est jamais
bloqué par un bug du hook.

Limites connues, assumées :
- `git push --no-verify` saute le hook (témoin P10) — les mandats l'interdisent déjà ; un filet côté
  superviseur (même contrôle refait sur `origin/main` à chaque cycle, alerte en cas d'écart) est
  prévu au lot 2 de #41 ;
- merge « truqué » : le contenu propre d'un commit de merge (résolution, ou modification ajoutée au
  moment du merge) n'est pas inspecté ; seul le rapport GO du second parent est exigé ;
- seul le second parent est contrôlé (un merge octopus n'est couvert que pour son deuxième parent) ;
- distant nul (premier push de la principale vers un distant) : la plage est `local --not --remotes`.
  « Publié » = atteignable depuis n'importe quelle ref `refs/remotes/*`, y compris une branche de
  travail déjà poussée — plus permissif que le cas nominal. Et sans aucune ref `refs/remotes/*`,
  **tout** l'historique de premier parent est examiné : le premier push d'un dépôt neuf est refusé
  dès qu'un commit direct touche hors `vault/` (témoin P20b) — l'amorçage se fait avant de poser
  `core.hooksPath` ;
- un commit vide (aucun chemin) sur la principale passe (il ne touche rien hors `vault/`) ;
- le hook n'est actif que sur les clones où `core.hooksPath` pointe ce dossier.

Témoins : `node --test scripts/harnais-hooks/pre-push.test.mjs` (`HOOK_PRE_PUSH=<chemin>` pour
rejouer la suite sur une autre version du hook) — chaque témoin fait un vrai `git push` vers un
distant nu jetable.
