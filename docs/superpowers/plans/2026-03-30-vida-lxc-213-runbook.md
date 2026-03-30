# Vida UI — Runbook LXC Cible `213`

**Date:** 2026-03-30  
**Contexte réel:** le conteneur `213` existe, son identité historique `OpenFange` a été remplacée par `vida-ai`, et le service headless est maintenant déployé dessus.

## 1. Décision d'exploitation

Le dépôt est désormais aligné sur ce cas réel:

- le script racine `install-lxc.sh` cible par défaut le `CT 213`
- le hostname cible est `vida-ai`
- le script réutilise un conteneur existant au lieu d'imposer une recréation

Objectif:

- transformer le `CT 213` vide en conteneur Vida AI headless exploitable
- cesser d'entretenir l'identité `OpenFange`

## 2. Effet attendu du script

Le script:

- détecte si le conteneur `213` existe déjà
- applique le hostname `vida-ai`
- démarre le conteneur si nécessaire
- installe Rust, Nginx et les dépendances build
- privilégie une archive locale `/tmp/vida-ai-src.tar.gz` si elle est présente sur l'hôte Proxmox
- sinon retombe sur le dépôt Git distant
- build `vida-headless --features remote`
- installe une unit `systemd` durcie
- configure `nginx` en frontal local
- applique une allowlist IP au niveau `nginx`
- applique du rate limiting et des headers de sécurité HTTP
- peut activer TLS en mode `provided` ou `self-signed`
- installe un timer `vida-ai-healthcheck.timer`
- fait binder Vida AI sur `127.0.0.1:3690`
- expose l'entrée HTTP via `nginx` sur `:80`
- redirige `:80` vers `:443` quand TLS est actif
- prépare un fichier firewall Proxmox `/etc/pve/firewall/213.fw`
- active `firewall=1` sur `net0` du CT

## 3. Commande recommandée

Depuis le dépôt local, pour déployer le code courant et non un snapshot GitHub plus ancien:

```bash
tar -czf /tmp/vida-ai-src.tar.gz \
  --exclude=.git \
  --exclude=target \
  --exclude=node_modules \
  --exclude=dist \
  --exclude=.svelte-kit \
  -C "/home/hackos0911/AI/projects/IA/Vida ui" .

scp /tmp/vida-ai-src.tar.gz root@192.168.50.29:/tmp/vida-ai-src.tar.gz
ssh root@192.168.50.29 'bash -s' -- 213 local-lvm vmbr20 < install-lxc.sh
```

Commande minimale si le script est exécuté directement sur l'hôte Proxmox:

```bash
bash install-lxc.sh 213 local-lvm vmbr0
```

Exemple avec TLS auto-signé:

```bash
VIDA_TLS_MODE=self-signed bash install-lxc.sh 213 local-lvm vmbr20
```

Exemple avec certificats fournis:

```bash
VIDA_TLS_MODE=provided \
VIDA_TLS_CERT_HOST_PATH=/tmp/vida-ai.crt \
VIDA_TLS_KEY_HOST_PATH=/tmp/vida-ai.key \
bash install-lxc.sh 213 local-lvm vmbr20
```

Exemple avec allowlist explicite:

```bash
VIDA_ALLOWLIST_CIDRS="127.0.0.1/32 192.168.20.0/24 192.168.50.0/24" \
bash install-lxc.sh 213 local-lvm vmbr20
```

## 4. Vérifications Proxmox avant exécution

Sur l'hôte Proxmox:

```bash
pct status 213
pct config 213
```

Points à vérifier:

- le conteneur `213` est bien celui à réutiliser
- il ne contient pas de charge utile à conserver
- il peut recevoir le hostname `vida-ai`

## 5. Renommage logique du conteneur

Le script applique:

```bash
pct set 213 --hostname vida-ai
```

et aligne aussi maintenant l'OS invité:

- `/etc/hostname`
- `/etc/hosts`
- `hostnamectl --static`

Si tu veux appliquer ce renommage manuellement avant le déploiement:

```bash
pct set 213 --hostname vida-ai
```

## 6. Architecture cible après déploiement

Dans le conteneur:

- Vida AI headless écoute sur `127.0.0.1:3690`
- `nginx` écoute sur `0.0.0.0:80`
- `nginx` sert une UI web distante statique sur `/`
- `nginx` applique une allowlist CIDR et un rate limit par IP
- `vida-ai-healthcheck.timer` sonde `http://127.0.0.1:3690/api/health` toutes les 5 minutes
- `vida-ai-soak-sample.timer` écrit des snapshots JSONL toutes les 5 minutes
- `systemd` supervise `vida-ai.service`
- les données persistent dans `/var/lib/vida-ai`

Chemins utiles:

- binaire: `/usr/local/bin/vida-ai` (installé depuis `target/release/vida-headless`)
- data dir: `/var/lib/vida-ai`
- token remote: `/var/lib/vida-ai/.token`
- logs app: `journalctl -u vida-ai`
- logs proxy: `journalctl -u nginx`
- health timer: `systemctl status vida-ai-healthcheck.timer`
- logs healthcheck: `journalctl -u vida-ai-healthcheck.service`
- soak timer: `systemctl status vida-ai-soak-sample.timer`
- log soak brut: `/var/log/vida-ai/soak-samples.jsonl`
- rapport soak: `vida-soak-report /var/log/vida-ai/soak-samples.jsonl`

## 7. Validation minimale après déploiement

Depuis l'hôte Proxmox:

```bash
pct exec 213 -- systemctl status vida-ai --no-pager
pct exec 213 -- systemctl status nginx --no-pager
pct exec 213 -- curl -sS http://127.0.0.1:3690/api/health
pct exec 213 -- curl -sS http://127.0.0.1/api/health
```

Depuis le réseau si le conteneur est joignable:

```bash
curl -sS http://<IP_DU_CT_213>/api/health
```

## 8. État validé au 2026-03-30

Validation réelle effectuée:

- `pct config 213` expose bien `hostname: vida-ai`
- `pct exec 213 -- hostnamectl --static` retourne `vida-ai`
- `pct config 213` expose `net0: ...,firewall=1,...`
- `/etc/pve/firewall/213.fw` existe et contient les règles `22/80/443` pour `192.168.20.0/24` et `192.168.50.0/24`
- `/etc/pve/firewall/cluster.fw` définit l'ipset `lan`
- `pve-firewall status` retourne `enabled/running`
- `systemctl is-active vida-ai nginx` retourne `active`
- `systemctl is-active vida-ai-soak-sample.timer` retourne `active`
- `curl -sS http://127.0.0.1:3690/api/health` retourne `status=ok`
- `curl -k -sS https://192.168.20.213/api/health` retourne `status=ok`
- `curl -sS http://192.168.20.213/api/health` retourne une redirection `301` vers HTTPS
- `curl -k -sS https://192.168.20.213/` retourne la page `Vida Remote Control`

## 9. Limites actuelles

- le filtrage réseau `nginx` est effectivement actif
- le firewall Proxmox est maintenant actif et le CT `213` est effectivement filtré par les règles cluster + CT
- TLS est actif en `self-signed`
- aucun certificat existant réutilisable `vida-ai` / `hackos.fr` n'a été trouvé sur l'hôte Proxmox
- pour une prod propre, il faut remplacer le certificat auto-signé par un certificat fourni
- le conteneur n'est pas encore validé sur une fenêtre complète `24-48h`, même si la collecte de soak est prête

## 10. Accès à l'interface web distante

Interface:

- `https://192.168.20.213/`

API santé:

- `https://192.168.20.213/api/health`

Pour utiliser l'interface:

1. récupérer le token service:

```bash
ssh root@192.168.50.29 "pct exec 213 -- cat /var/lib/vida-ai/.token"
```

2. ouvrir la page web
3. coller le token dans `Service token`
4. faire `Bootstrap` si aucun utilisateur n'existe encore, sinon `Login`

Capacités actuellement exposées par l'UI web:

- bootstrap / login / logout
- health public et admin
- gestion utilisateurs
- providers runtime
- création, listing et suppression de sessions
- envoi chat simple et streaming WebSocket
- lecture de l'historique des messages
- création, listing et inspection des équipes
- création de session d'équipe
- lecture des audits

## 11. Étape suivante recommandée

Après déploiement réussi sur `213`:

1. remplacer le certificat `self-signed` par un certificat fourni
2. mettre en place monitoring / alerting
3. laisser tourner la collecte `24-48h` sur ce conteneur précis
4. produire un rapport à partir de `vida-soak-report`
