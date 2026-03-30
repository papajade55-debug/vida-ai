# Vida UI — Protocole d'Authentification Remote

**Date:** 2026-03-30  
**Statut:** implémenté dans `crates/vida-core/src/remote.rs`  
**Portée:** mode headless / API HTTP + WebSocket

## 0. Exposition réseau recommandée

Le protocole remote est prévu pour être exposé derrière un frontal `nginx` ou équivalent:

- backend Vida AI sur `127.0.0.1:3690`
- frontal HTTP/TLS sur `:80` et éventuellement `:443`
- allowlist CIDR côté frontal
- rate limiting côté frontal

Même avec le double mécanisme `service token + session utilisateur`, l'API remote ne doit pas être exposée brute sur Internet sans frontal TLS et filtrage réseau.

## 1. Objectif

Le mode remote utilise désormais une authentification à deux niveaux:

- **token de service remote** pour exposer l'API sur le réseau
- **session utilisateur distante** pour identifier l'acteur humain et appliquer les rôles `super_admin`, `architect`, `operator`

Ce découpage évite deux erreurs fréquentes:

- exposer directement l'API métier derrière un simple token statique
- réutiliser `current_actor` global du runtime local, ce qui casserait l'isolation entre clients HTTP concurrents

## 2. Modèle de sécurité

### 2.1 Niveau 1: token de service

Toutes les routes `/api/*` sauf `/api/health` exigent un token de service remote.

Méthodes acceptées:

- header `Authorization: Bearer <remote-token>`
- header `x-vida-service-token: <remote-token>`
- query string `?service_token=<remote-token>` uniquement pour compatibilité limitée, surtout utile au WebSocket

Sources du token:

- Tauri GUI via `get_remote_token`
- headless via secret store `remote-api-token`
- génération via `VidaEngine::generate_remote_token()`

### 2.2 Niveau 2: session utilisateur

Les routes métier exigent aussi une session utilisateur distante.

Méthodes acceptées:

- header `x-vida-session: <session-token>`
- query string `?session_token=<session-token>` pour le WebSocket

Le `session_token` est émis par:

- `POST /api/auth/bootstrap`
- `POST /api/auth/login`

La session est actuellement:

- stockée **en mémoire**
- avec un **TTL de 12 heures**
- invalide après redémarrage du serveur remote
- invalidée explicitement par `POST /api/auth/logout`

### 2.3 Anti-bruteforce login

Le endpoint `POST /api/auth/login` applique un rate limit en mémoire par utilisateur:

- fenêtre de comptage: **5 minutes**
- seuil: **5 échecs**
- blocage: **15 minutes**

En cas de dépassement:

- statut HTTP: `429 Too Many Requests`
- un événement d'audit `remote.auth.rate_limited` est enregistré

### 2.4 Audit

Les événements sensibles remote sont journalisés en base SQLite dans `audit_events`.

Le runtime remote émet aussi des logs structurés JSON sur `stderr` avec:

- `ts`
- `level`
- `component`
- `event`
- `fields`

Événements actuellement émis:

- `remote.auth.bootstrap`
- `remote.auth.bootstrap_failed`
- `remote.auth.login`
- `remote.auth.login_failed`
- `remote.auth.rate_limited`
- `remote.auth.logout`
- `remote.auth.change_password`
- `remote.admin.user_create`

Les enregistrements contiennent:

- `actor_username`
- `actor_role`
- `event_type`
- `resource`
- `details_json`
- `created_at`

## 3. Endpoints

### 3.1 Public

#### `GET /api/health`

Retourne l'état minimal du service.

Exemple:

```bash
curl -sS http://127.0.0.1:3690/api/health
```

Réponse:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_seconds": 1234,
  "remote_session_ttl_seconds": 43200
}
```

### 3.2 Auth remote

#### `GET /api/auth/status`

Retourne:

- `has_users`
- `actor` si une session remote valide est fournie

Exemple:

```bash
curl -sS \
  -H "Authorization: Bearer $VIDA_REMOTE_TOKEN" \
  -H "x-vida-session: $VIDA_SESSION_TOKEN" \
  http://127.0.0.1:3690/api/auth/status
```

Réponse:

```json
{
  "has_users": true,
  "actor": {
    "user_id": "uuid",
    "username": "admin.local",
    "role": "super_admin"
  }
}
```

#### `POST /api/auth/bootstrap`

Autorisé seulement quand aucun utilisateur local n'existe encore.

Payload:

```json
{
  "username": "admin.local",
  "password": "supersecret"
}
```

Réponse:

```json
{
  "session_token": "vida_...",
  "actor": {
    "user_id": "uuid",
    "username": "admin.local",
    "role": "super_admin"
  }
}
```

#### `POST /api/auth/login`

Payload:

```json
{
  "username": "admin.local",
  "password": "supersecret"
}
```

Réponse:

```json
{
  "session_token": "vida_...",
  "actor": {
    "user_id": "uuid",
    "username": "admin.local",
    "role": "super_admin"
  }
}
```

#### `POST /api/auth/logout`

Invalide la session courante.

Exemple:

```bash
curl -i -sS \
  -X POST \
  -H "Authorization: Bearer $VIDA_REMOTE_TOKEN" \
  -H "x-vida-session: $VIDA_SESSION_TOKEN" \
  http://127.0.0.1:3690/api/auth/logout
```

Réponse attendue:

- `204 No Content`

#### `POST /api/auth/change-password`

Change le mot de passe de l'utilisateur authentifié.

Payload:

```json
{
  "current_password": "old-secret",
  "new_password": "new-secret"
}
```

Réponse attendue:

- `204 No Content`

### 3.3 Admin remote minimal

Ces routes exigent:

- token de service remote valide
- session utilisateur valide
- rôle `super_admin`

#### `GET /api/admin/users`

Retourne la liste des utilisateurs locaux.

Exemple:

```bash
curl -sS \
  -H "Authorization: Bearer $VIDA_REMOTE_TOKEN" \
  -H "x-vida-session: $VIDA_SESSION_TOKEN" \
  http://127.0.0.1:3690/api/admin/users
```

#### `GET /api/admin/health`

Retourne l'état d'exploitation du serveur remote.

Champs principaux:

- `uptime_seconds`
- `has_users`
- `active_sessions`
- `rate_limited_users`
- `audit_event_count`
- `latest_audit_at`
- `provider_count`
- `mcp_tool_count`

#### `GET /api/admin/audit`

Retourne les événements d'audit avec filtres simples.

Query params supportés:

- `limit` entre `1` et `200`
- `actor_username`
- `event_type`
- `created_after`

Exemple:

```bash
curl -sS \
  -H "Authorization: Bearer $VIDA_REMOTE_TOKEN" \
  -H "x-vida-session: $VIDA_SESSION_TOKEN" \
  "http://127.0.0.1:3690/api/admin/audit?event_type=remote.auth.login_failed&limit=50"
```

#### `POST /api/admin/users`

Crée un utilisateur humain additionnel.

Restriction volontaire actuelle:

- rôles autorisés à la création via remote: `architect`, `operator`
- création de `super_admin` via remote: refusée
- création de `agent` via remote: refusée

Payload:

```json
{
  "username": "arch.local",
  "password": "architect1",
  "role": "architect"
}
```

Réponse:

```json
{
  "id": "uuid",
  "username": "arch.local",
  "role": "architect",
  "active": true,
  "created_at": "2026-03-30T..."
}
```

## 4. Règles de rôle appliquées

### 4.1 Routes métier remote

Autorisés:

- `super_admin`
- `architect`
- `operator`

Refusé:

- `agent`

Routes concernées:

- `GET /api/providers`
- `GET /api/sessions`
- `POST /api/sessions/create`
- `POST /api/chat/send`
- `GET /api/chat/stream`

### 4.2 Routes admin remote

Autorisé:

- `super_admin`

Refusés:

- `architect`
- `operator`
- `agent`

## 5. Exemples de séquence complète

### 5.1 Bootstrap initial

```bash
REMOTE_TOKEN="$(cat ~/.vida-ai/.token)"

BOOTSTRAP_JSON="$(
  curl -sS \
    -X POST \
    -H "Authorization: Bearer $REMOTE_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin.local","password":"supersecret"}' \
    http://127.0.0.1:3690/api/auth/bootstrap
)"
```

### 5.2 Login standard

```bash
LOGIN_JSON="$(
  curl -sS \
    -X POST \
    -H "Authorization: Bearer $REMOTE_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin.local","password":"supersecret"}' \
    http://127.0.0.1:3690/api/auth/login
)"
```

### 5.3 Appel métier

```bash
curl -sS \
  -H "Authorization: Bearer $REMOTE_TOKEN" \
  -H "x-vida-session: $VIDA_SESSION_TOKEN" \
  http://127.0.0.1:3690/api/providers
```

### 5.4 WebSocket streaming

Le WebSocket accepte les tokens en query string:

```text
ws://127.0.0.1:3690/api/chat/stream?service_token=<remote-token>&session_token=<session-token>
```

Premier message attendu:

```json
{
  "session_id": "session-uuid",
  "content": "Bonjour"
}
```

Événements émis:

- `{"type":"token","content":"..."}`
- `{"type":"error","error":"..."}`
- `{"type":"done"}`

## 6. Limites actuelles

- les sessions remote sont **non persistées**
- les limites de login sont **en mémoire** et donc réinitialisées au redémarrage
- pas encore d'API remote pour désactiver un utilisateur ou changer son rôle

## 7. Recommandations immédiates

- ajouter une persistance optionnelle des sessions remote ou un store Redis si le mode cluster apparaît
- ajouter des filtres plus riches sur les audits si le volume augmente
- ajouter un rate limit aussi sur `bootstrap`
- ne pas exposer l'API remote sans reverse proxy, TLS et filtrage IP en LXC prod
