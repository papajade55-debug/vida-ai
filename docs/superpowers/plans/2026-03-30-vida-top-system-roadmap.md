# Vida UI — Plan Mis à Jour Vers un Système Production-Grade

**Date:** 2026-03-30  
**Auteur:** Codex  
**Objectif:** consolider ce qui est déjà livré, identifier précisément ce qui reste à faire, et fournir un ordre d'exécution clair pour obtenir un système robuste, sécurisé, observable et exploitable en production.

## 1. État Global

### 1.1 Niveau de complétion

- **Plan de correction technique engagé:** ~`85%`
- **Préparation production complète:** ~`70-75%`

### 1.2 Lecture synthétique

Le socle applicatif est désormais solide:

- auth locale réelle
- auth remote réelle
- RBAC équipes
- hiérarchie d'acteurs `SuperAdmin -> Architect -> Operator -> Agent`
- sandbox stricte pour les agents IA
- HITL pour les opérations sensibles
- bootstrap runtime providers
- boucle tool-calling backend
- support natif `tool_calls` OpenAI / Anthropic / Google
- remote/headless durci avec session utilisateur, TTL, rate limiting login, audit persistant, lecture admin d'audit, health admin et logs structurés

Le reliquat n'est plus du "cœur fonctionnel". Ce qui reste est principalement:

- hardening production LXC
- observabilité et exploitation
- tests de charge et soak réels
- finitions d'UX et client remote
- validation cloud réelle avec vraies clés et vrais environnements

## 2. Ce Qui Est Déjà Fait

## 2.1 Authentification et rôles

Statut: `livré`

- utilisateurs applicatifs persistants
- bootstrap du premier `super_admin`
- login/logout local
- changement de mot de passe
- création d'utilisateurs humains (`architect`, `operator`)
- propagation des rôles dans les contrôles d'accès backend

## 2.2 Hiérarchie d'accès

Statut: `livré`

- politique centralisée `SuperAdmin / Architect / Operator / Agent`
- refus explicite des actions non autorisées
- approbation humaine obligatoire sur les actions critiques
- verrouillage des agents IA hors sandbox

## 2.3 Sandboxing agent

Statut: `livré`

- agents IA limités à la lecture projet
- écriture autorisée uniquement dans `.vida/sandboxes/...`
- shell / escalation interdits aux agents
- interdiction de toucher au code critique et à la config sensible

## 2.4 Providers et tool-calling

Statut: `majoritairement livré`

- providers `ollama`, `openai`, `anthropic`, `google` chargés au runtime
- boucle backend `LLM -> tool -> result -> LLM`
- validation d'arguments d'outils
- support natif provider des `tool_calls` pour OpenAI-compatible, Anthropic et Google

## 2.5 Permissions et HITL

Statut: `livré`

- flux `Ask` backend/frontend restauré
- approbation humaine pour opérations sensibles
- raccordement aux commandes critiques Tauri / MCP

## 2.6 Teams / RBAC équipes

Statut: `livré sur le périmètre actuel`

- rôles `owner/admin/member/viewer`
- protection contre la perte du dernier `owner`
- `viewer` exclu de l'exécution active

## 2.7 Remote / Headless

Statut: `livré et durci`

- commandes remote réellement branchées
- auth remote à deux niveaux: token service + session utilisateur
- sessions remote avec TTL `12h`
- rate limiting login: `5` échecs / `5 min`, blocage `15 min`
- API admin remote minimale
- lecture filtrable des `audit_events`
- health admin enrichi
- logs structurés JSON côté remote/headless
- audit persistant SQLite des événements auth/admin

## 2.8 Base de données / audit

Statut: `livré`

- migration `005_auth.sql`
- migration `006_audit.sql`
- persistance utilisateurs
- persistance audit events

## 2.9 Scripts et packaging

Statut: `partiellement livré`

- script LXC racine corrigé pour builder le bon binaire
- cohérence meilleure qu'au point de départ
- hardening système encore incomplet

## 3. Ce Qui Reste à Faire

## 3.1 Priorité P0 — Pour un système "au top" en production

### A. Observabilité et exploitation

Statut: `partiellement fait`

Déjà fait:

- endpoint admin de lecture des `audit_events`
- logs structurés remote/headless
- healthcheck enrichi:
  - `/api/health`
  - `/api/admin/health`

À livrer:

- métriques runtime minimales:
  - succès/échec login
  - sessions actives
  - tool calls exécutés
  - erreurs providers
  - latence chat / tool-call
- healthcheck enrichi
- stratégie d'alerting

Critère de sortie:

- un opérateur peut diagnostiquer un incident sans accès direct au code

### B. Hardening LXC production

Statut: `partiellement fait`

Déjà fait:

- script LXC unifié sur le cas réel `CT 213`
- réutilisation d'un conteneur existant vide au lieu d'imposer une recréation
- bind headless configurable avec usage prévu sur `127.0.0.1`
- `nginx` local devant Vida AI dans le script de déploiement
- unit `systemd` durcie
- runbook dédié pour le conteneur `213`
- binaire headless dédié sans dépendance Tauri GUI pour LXC
- déploiement réel validé sur le `CT 213`
- renommage effectif `openfang -> vida-ai` côté Proxmox et dans l'OS invité
- healthcheck validé sur `127.0.0.1:3690` et `http://192.168.20.213/api/health`
- allowlist CIDR et rate limiting côté `nginx`
- support TLS optionnel `provided` / `self-signed`
- timer `systemd` de healthcheck périodique
- HTTPS réel validé sur `https://192.168.20.213/api/health`
- fichier firewall Proxmox `213.fw` généré et `net0 firewall=1`
- ipset cluster `lan` défini et `pve-firewall` réactivé
- firewall Proxmox réellement effectif pour le `CT 213`
- collecteur de soak JSONL et rapporteur installables dans le CT

À livrer:

- remplacement du certificat `self-signed` par un vrai certificat
- séparation claire:
  - data dir
  - logs
  - secrets
  - sandboxes

Critère de sortie:

- l'exposition réseau du remote n'est plus "direct process", mais "service encapsulé et contrôlé"

### C. Validation réelle providers cloud

Statut: `à faire`

À livrer:

- tests runtime réels avec vraies clés:
  - OpenAI
  - Anthropic
  - Google
- mesure de:
  - latence
  - taux de succès
  - comportement erreurs
  - comportement timeouts
- tests de fallback quand un provider est indisponible

Critère de sortie:

- chaque provider annoncé a une validation de prod ou est explicitement rétrogradé en "expérimental"

### D. Soak / charge

Statut: `en cours`

Déjà fait:

- collecteur périodique `vida-ai-soak-sample.timer` déployé sur le `CT 213`
- snapshots JSONL dans `/var/log/vida-ai/soak-samples.jsonl`
- rapporteur local `vida-soak-report`
- premier échantillonnage réel validé:
  - `sample_count=4`
  - `health_success_rate_pct=100.0`
  - `vida_rss_kb=18436..18460`
  - `db_bytes=135168`

À livrer:

- test `24-48h` headless
- charge concurrente sessions / équipes / tool-calls
- suivi mémoire / CPU / descripteurs / taille DB
- vérification absence de fuite session remote
- vérification rotation correcte des sandboxes et stabilité MCP

Critère de sortie:

- aucune dégradation non contrôlée sur une fenêtre longue

## 3.2 Priorité P1 — Pour l'exploitabilité premium

### E. Audit admin exploitable

Statut: `partiellement fait`

Déjà fait:

- stockage des audits

Reste:

- lecture admin des audits
- filtres par acteur / type / date
- export minimal
- UI sécurité/admin ou endpoint remote dédié

### F. Client remote / UX distante

Statut: `à faire`

À livrer:

- client remote consommant le protocole HTTP/WS réel
- gestion login/logout/session expirée
- feedback explicite sur `429` login rate-limited
- UI admin pour utilisateurs et audits

### G. Gestion du cycle de vie session remote

Statut: `partiellement fait`

Déjà fait:

- TTL en mémoire
- logout

Reste:

- éventuellement refresh explicite
- expiration visible côté client
- éventuellement révocation admin

## 3.3 Priorité P2 — Excellence technique

### H. Politique de secrets et rotation

Statut: `à renforcer`

À livrer:

- documentation d'exploitation des secrets
- rotation remote token
- rotation API keys providers
- audit des modifications sensibles

### I. Politique fine de code critique

Statut: `bon socle, améliorable`

À livrer:

- définition plus formelle des répertoires/fichiers "critical code"
- tests dédiés sur ces frontières

### J. Outillage de release

Statut: `à faire`

À livrer:

- procédure de release
- checklist de déploiement
- rollback documenté
- validation post-déploiement

## 4. Plan d'Exécution Recommandé

## Phase 1 — Finir le hardening opérationnel

Durée estimée: `1 à 3 jours`

Livrer:

- lecture admin des audits
- logs structurés
- healthcheck enrichi
- docs d'exploitation remote

## Phase 2 — Sécuriser l'enveloppe LXC

Durée estimée: `1 à 3 jours`

Livrer:

- reverse proxy
- TLS
- firewall
- unit/service hardening
- chemins RW strictement limités

## Phase 3 — Valider les providers réels

Durée estimée: `1 à 2 jours`

Livrer:

- tests runtime réels OpenAI/Anthropic/Google
- matrice de support prod

## Phase 4 — Charge et soak

Durée estimée: `2 à 3 jours`

Livrer:

- test `24-48h`
- rapport de stabilité
- correctifs post-soak

## Phase 5 — UX admin/remote premium

Durée estimée: `1 à 3 jours`

Livrer:

- lecture audits côté interface
- gestion session expirée
- UX login/admin propre

## 5. Définition d'un "Système au Top"

Le système pourra être considéré "au top" quand les conditions suivantes seront simultanément vraies:

- auth locale et remote stables
- rôles réellement appliqués
- agents IA strictement confinés
- providers réels validés en runtime
- remote protégé par reverse proxy + TLS + filtrage IP
- audits lisibles et exploitables
- métriques et alertes disponibles
- soak `24-48h` réussi
- procédure de déploiement et rollback documentée

## 6. Risques Restants

- sans validation cloud réelle, une partie du support providers reste "implémentée mais non prouvée"
- sans soak longue durée, la stabilité mémoire et session reste partiellement théorique
- sans lecture admin des audits, l'audit existe mais n'est pas encore pleinement exploitable
- sans reverse proxy/TLS/firewall, le remote ne doit pas être exposé tel quel sur un réseau non maîtrisé

## 7. Recommandation Finale

Le projet a dépassé le stade "prototype local". Il est désormais dans une phase **pré-production avancée**.

La meilleure stratégie n'est plus d'ajouter beaucoup de nouvelles fonctionnalités. La priorité est maintenant:

1. **observabilité**
2. **hardening LXC / réseau**
3. **validation réelle providers**
4. **soak et charge**
5. **UX admin de finition**

Si cet ordre est respecté, Vida UI peut converger vers une mise en production sérieuse avec un risque maîtrisé.
