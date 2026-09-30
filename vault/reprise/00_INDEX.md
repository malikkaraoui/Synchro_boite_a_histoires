---
date: 2026-09-30
tags: [reprise, sessions, tableau-de-bord]
maintenu_par: orchestrateur
derniere_maj: 2026-09-30T12:54:15+0200
---

# Tableau de bord — sessions du projet synchro-boite

<!-- rotation-index -->
> **Index court (rotation).** Seuls les derniers jours d'activité sont ici. L'historique complet
> est dans `vault/reprise/archive/index/00_INDEX-AAAA-MM.md` — le chercher par mots-clés, ne
> jamais le charger en entier. Le hook `pre-commit` livré avec ce scaffold refuse un commit qui
> ferait dépasser la limite d'octets à ce fichier, et imprime le remède : c'est le seul garde-fou
> contre un index de plusieurs centaines de kilo-octets relu à chaque démarrage de session.

<!--
CONVENTION D'ÉCRITURE — une entrée par mandat ou revue traité, ordre ANTI-CHRONOLOGIQUE
(le plus récent en tête, immédiatement sous ce bloc). Format d'une entrée :

## <horodatage ISO 8601 avec fuseau> — <Mxxxx|R0xx> (<Fxx>) : <verdict en UNE phrase>

- <fait vérifiable, avec son pointeur : rapport, branche, SHA — jamais un ressenti>
- <ce qui est mergé / non mergé, et ce qui bloque, nommé>
- <réserve ou trou assumé, s'il y en a un — l'absence de réserve se dit aussi>
- <leçon ou décision rattachée, par lien>

Règles :
- Le titre H2 porte le VERDICT, pas le sujet : il doit se lire seul, sans ouvrir le rapport.
- 2 à 4 puces, pas davantage : ce fichier est un index, pas un rapport.
- Jamais de narration : ce qui mérite d'être raconté va dans `vault/revues/`, et l'entrée pointe.
- Cette entrée est une DÉCLARATION, pas une preuve (voir `vault/runtime/README.md`).

Exemple (à supprimer à la première entrée réelle) :

## AAAA-MM-JJTHH:MM:SS+00:00 — <R0xx> (<Fxx>) doublage de <Mxxxx> : GO, mergé <sha court>

- Rapport : `vault/revues/AAAA-MM-JJ-<R0xx>-<slug>.md` ; branche `<branche>` (tip `<sha>`).
- Symptôme d'origine rejoué avant/après ; tests re-mesurés par le doubleur, pas repris du rapport d'auteur.
- Réserve non bloquante : <réserve nommée, ou « aucune »>.
-->

## 2026-09-30T14:24:38+0200 — M0003 (F01) : socle App Store vert (46/46, build release), audit G1–G10 + E1–E11 livré

- Rapport : `vault/echanges/archive/2026-09-30-F01-M0003-appstore-rust-socle-et-audit.md` ; audit `vault/revues/2026-09-30-M0003-audit-app-store.md`.
- Poussé : `chore/M0002-harnais-storybox`=a9cb2f3, `feat/M0003-appstore-rust`=a809030 (vérifié refs origin). Non mergé : doublage R001 à faire.
- Bloquants mesurés : G2 sandbox (lecture/écriture boîte EPERM, faux positif « connectée »), E1 build App Store cassé (feature supprimée encore citée), G7 aucun certificat Distribution.
- Leçon : l'hypothèse orchestrateur de M0002 sur les droits (`commit --only`) était fausse — cause du STOP M0002.

## 2026-09-30T12:54:15+0200 — M0001 (F01) : superviseur activé pour synchro-boite, config complétée

- Rapport : `vault/echanges/archive/2026-09-30-F01-M0001-activation-superviseur.md`. Aucun git (dérogation voulue).
- Contre-vérifié par l'orchestrateur : config `actif=true`, `doctrine_projet=[".claude/CLAUDE.md"]` ; projet présent dans `~/.harnais/config.json` ; `SUPERVISEUR_DEMARRE` 12:49:24 dans harnais.log et events.jsonl.
- Trou assumé : `launchctl print` jamais collé (outil refusé à la session) ; 1er `launchctl bootstrap` en EIO, 2e réussi.
- Constat : aucun superviseur harnais entre 01:42 (SIGTERM) et 12:49 le 2026-09-30 pour les 3 autres projets.

