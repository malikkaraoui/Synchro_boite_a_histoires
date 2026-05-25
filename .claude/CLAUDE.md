# CLAUDE.md — Synchro_boite_a_histoires

## §0 Contexte projet actif

| Clé | Valeur |
| --- | --- |
| Projet | Synchro_boite_a_histoires |
| Phase | — |
| Stack | — |

## §1 Horodatage

Réponse DOIT commencer par : `[YYYY-MM-DD HH:MM:SS | MODEL-ID]`

## §2 Langue & Ton

Français. Direct. Actionnable.

## §5 Anti-hallucination

Interdit d'inventer. Si incertain → « Je ne peux pas l'affirmer » + hypothèses.
## §vault Vault projet — obligation non négociable

**Vault-first** : toute question sur l'état du projet (fonctionnalité active ? livré ? testé ? décidé ?) → lire `vault/30-discoveries.md` avant de répondre. Répondre sans lire = interdit.

**Mise à jour automatique** (sans attendre instruction) :
- Fin de session significative → MAJ `vault/00-brief.md` (sync §0) + entrée dans `vault/10-mailbox.md`
- Décision prise → `vault/20-decisions.md`
- Découverte → `vault/30-discoveries.md`
- Avancement ou nouvelle tâche → `vault/40-roadmap.md`

Le vault est la mémoire vivante du projet, exposée dans Obsidian via symlink. Chaque session qui ne laisse pas de trace dans le vault est une session perdue.
