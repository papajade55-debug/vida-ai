# Plan detaille - Faire de Vida UI un vrai systeme d'equipes

Date: 2026-03-30
Portee: transformer Vida UI en systeme d'equipes multi-agents reellement exploitable, stable, gouvernable et testable

## 1. Objectif

Construire dans Vida UI un vrai systeme d'equipes, pas seulement un affichage de plusieurs agents.

Le resultat cible est:
- creation d'equipes persistantes
- membres avec roles explicites
- sessions d'equipe dediees
- orchestration multi-agents deterministe
- outils et MCP sous controle de permissions
- historique, traces, audit et reprise
- interface utilisable en continu

Le produit final doit permettre a un utilisateur de:
1. creer une equipe
2. choisir les agents et leurs roles
3. lancer une session equipe
4. envoyer une demande unique
5. voir les agents travailler en parallele ou en pipeline
6. voir un coordinateur produire une synthese finale
7. controler les permissions et les outils utilises
8. rejouer, auditer et evaluer le resultat

## 2. Constat de depart

Base deja presente dans Vida UI:
- commandes Tauri teams exposees
- create/list/get/delete team
- session d'equipe
- streaming d'equipe
- hooks frontend teams
- mode chat equipe
- slices store pour teams et streaming agent
- MCP manager present
- workspace et permissions presents

Base manquante ou partielle:
- boucle agent autonome `agent_loop` non livree
- orchestration LLM ↔ MCP incomplete
- roles d'equipe encore faibles
- gouvernance permissions pas completement branchee sur MCP/file/shell/network
- peu de garanties de stabilite de session equipe
- pas encore de vrai superviseur ou coordinateur robuste
- pas encore de tests e2e complets sur scenario equipe

Conclusion:
- la direction architecturale est bonne
- il faut faire du build produit et du hardening
- il ne faut pas repartir de zero

## 3. Definition d'un "vrai systeme d'equipes"

Vida ne devra pas se contenter de lancer plusieurs appels LLM.

Un vrai systeme d'equipes implique au minimum:

### 3.1 Structure
- une equipe a un identifiant, un nom, un objectif optionnel, une politique de travail
- une equipe contient des membres
- un membre possede:
  - provider
  - modele
  - role
  - couleur / identite UI
  - permissions
  - outils disponibles
  - comportement de coordination

### 3.2 Roles
- `coordinator`
- `researcher`
- `coder`
- `critic`
- `planner`
- `executor`
- `reviewer`

Chaque role doit avoir:
- instructions systeme propres
- budget de contexte
- budget de tokens
- politique outils

### 3.3 Runtime
- une demande utilisateur ouvre une boucle equipe
- le coordinateur decompose
- les membres executent
- les resultats remontent
- le coordinateur integre
- le systeme cloture proprement

### 3.4 Gouvernance
- chaque action sensible passe par PermissionManager
- les outils ont des gates
- les decisions importantes sont tracees
- l'utilisateur peut interrompre, rejouer, approuver ou refuser

### 3.5 Observabilite
- timeline d'execution
- traces par agent
- outils invoques
- cout
- duree
- etat final

## 4. Cible produit

## 4.1 MVP fort

Le premier jalon exploitable doit livrer:
- CRUD complet des equipes
- membres et roles persistants
- session equipe
- coordinateur simple
- execution sequentielle ou semi-parallele
- streaming UI par agent
- synthese finale
- outils MCP et shell/filesystem sous permission gating
- historique DB
- tests end-to-end sur 3 scenarios

## 4.2 Version mature

La version mature ajoute:
- parallelisme reelement stable
- politiques avancées d'equipe
- quotas et limites
- templates d'equipe
- comparateur de fournisseurs
- reprise de session interrompue
- remote/headless
- scoring et evaluation automatique

## 5. Architecture cible

## 5.1 Couches

### Frontend
- ecrans teams
- creation / edition
- session equipe
- streaming multi-colonnes ou timeline
- panneau audit / outils / permissions

### Tauri commands
- commandes teams
- commandes sessions equipe
- commandes permissions
- commandes MCP
- commandes providers

### Engine
- orchestration de haut niveau
- decomposition des taches
- plan d'execution equipe
- appel des providers
- stockage et reprise

### Runtime agent
- boucle agent
- tool calling
- limite d'iterations
- gestion erreurs / retries
- emission d'evenements

### DB
- equipes
- membres
- sessions
- messages
- traces
- executions outils
- decisions permissions

## 5.2 Pattern d'orchestration recommande

Pattern cible:
- orchestrateur central + agents specialises

Pourquoi:
- plus simple a deboguer
- plus lisible en UI
- meilleur controle du cout
- plus facile a gouverner

Pipeline recommande:
1. coordinator lit la demande
2. coordinator genere un plan
3. dispatcher assigne les sous-taches
4. agents executent
5. reviewer critique
6. coordinator synthese et livre

Au debut:
- parallelisme limite a 2-4 agents simultanes
- temps d'execution borne
- nombre de tours borne

## 6. Modele de donnees cible

## 6.1 Tables minimales

### teams
- id
- name
- description
- owner_id optionnel
- policy_json
- created_at
- updated_at

### team_members
- id
- team_id
- member_name
- role
- provider
- model
- system_prompt
- color
- enabled
- tool_policy_json
- created_at

### team_sessions
- id
- team_id
- user_goal
- status
- coordinator_member_id
- created_at
- completed_at

### team_session_events
- id
- session_id
- event_type
- actor_member_id nullable
- payload_json
- created_at

### team_messages
- id
- session_id
- member_id nullable
- role
- content
- metadata_json
- created_at

### tool_executions
- id
- session_id
- member_id
- tool_name
- input_json
- output_json
- success
- latency_ms
- created_at

### permission_decisions
- id
- session_id
- member_id nullable
- action
- resource
- decision
- reason
- created_at

## 6.2 Contraintes
- suppression d'equipe cascade sur membres et sessions
- index sur `team_id`, `session_id`, `created_at`
- stockage JSON pour politiques et traces

## 7. Phases de livraison

## Phase 1 - Solidifier les equipes existantes

But:
- rendre fiable ce qui existe deja en surface

Travail:
- auditer les commandes `teams`
- verifier migrations SQL et schema
- verifier coherence frontend/store/backend
- ajouter validations serveur:
  - nom equipe non vide
  - roles valides
  - membre unique par equipe
  - au moins un coordinateur ou leader
- durcir les erreurs et messages UI

Sortie attendue:
- CRUD teams stable
- sessions equipe creees sans incoherence

Critere d'acceptation:
- tests DB teams passent
- UI permet creer, lister, ouvrir, supprimer
- roles persistants et relus correctement

## Phase 2 - Definir les roles et la politique equipe

But:
- sortir d'un simple groupe d'agents sans contrat

Travail:
- definir enum roles stable
- definir prompts systeme par role
- definir matrice outils par role
- definir politiques:
  - `strict`
  - `balanced`
  - `full-access`
- definir budgets:
  - max_iterations
  - max_tokens
  - max_tool_calls

Sortie attendue:
- un membre d'equipe a un comportement previsible

Critere d'acceptation:
- l'utilisateur choisit role + provider + modele
- le moteur charge le prompt et la policy appropries

## Phase 3 - Livrer le runtime d'equipe

But:
- avoir une vraie execution multi-agents

Travail:
- implementer `agent_loop`
- creer `team_runtime.rs` ou equivalent
- definir les evenements:
  - `TeamStarted`
  - `PlanCreated`
  - `AgentStarted`
  - `AgentToken`
  - `AgentToolCall`
  - `AgentToolResult`
  - `AgentFailed`
  - `ReviewProduced`
  - `FinalAnswer`
  - `AllDone`
- supporter deux strategies:
  - sequentielle
  - parallele bornee

Premiere version recommandee:
- coordinateur planifie
- researcher + coder + critic executent
- coordinateur synthese

Critere d'acceptation:
- pour un prompt simple, 3 agents produisent chacun une sortie
- une synthese finale est produite
- aucun deadlock

## Phase 4 - Tool calling et MCP reels

But:
- rendre l'equipe utile, pas seulement bavarde

Travail:
- brancher la boucle agent sur MCP manager
- ajouter routeur d'outils commun
- normaliser les schemas d'appel
- gerer les erreurs outils
- ajouter timeouts, retries, circuit breaker
- limiter les outils disponibles par role

Priorite outils:
1. filesystem
2. shell
3. web_fetch
4. web_search
5. MCP custom

Critere d'acceptation:
- un agent peut appeler un outil MCP
- le resultat revient dans la boucle
- l'equipe termine proprement

## Phase 5 - PermissionManager partout

But:
- gouverner reellement le systeme

Travail:
- brancher `PermissionManager` sur:
  - shell
  - filesystem
  - network/web
  - MCP start/stop
  - MCP tool call
  - team create/update/delete
  - provider key usage sensible
- ajouter modes:
  - `ask`
  - `allow-listed`
  - `full`
- historiser chaque decision

Critere d'acceptation:
- aucune action sensible ne bypass le gate
- journal de decision consultable

## Phase 6 - UX de session equipe

But:
- rendre le multi-agents lisible

Travail:
- vue conversation equipe
- colonnes ou cards par agent
- timeline des etapes
- etat live:
  - planning
  - running
  - waiting_tool
  - blocked_permission
  - done
  - failed
- panneau final avec:
  - reponse finale
  - contributions par agent
  - outils utilises
  - cout
  - duree

Critere d'acceptation:
- l'utilisateur comprend qui fait quoi
- l'utilisateur voit quand et pourquoi une execution bloque

## Phase 7 - Stabilite, persistence et reprise

But:
- en faire un runtime utilisable tous les jours

Travail:
- checkpoint session
- reprise sur crash
- annulation propre
- backpressure streaming
- dedup d'evenements
- timeouts par agent
- retries bornes
- supervision du coordinateur

Critere d'acceptation:
- une fermeture/reouverture de l'app ne corrompt pas la session
- une erreur agent n'empeche pas la synthese finale si une degradation est possible

## Phase 8 - Evaluation et qualite

But:
- mesurer si le systeme equipe est meilleur qu'un simple agent

Travail:
- jeux de prompts de reference
- scenarios:
  - recherche
  - planification
  - code
  - synthese
  - execution outillee
- metriques:
  - temps
  - cout
  - taux d'echec
  - qualite percue
  - nombre de tool calls
- comparer:
  - solo agent
  - team 2 agents
  - team 4 agents

Critere d'acceptation:
- gains visibles ou au minimum absence de regression critique

## 8. Decisions techniques recommandees

## 8.1 Ne pas faire
- ne pas laisser tous les agents acceder a tous les outils
- ne pas faire de parallelisme massif des le debut
- ne pas melanger orchestration equipe et UI dans le meme code
- ne pas rendre le coordinator omnipotent sans limites

## 8.2 Faire
- centraliser la logique equipe dans le moteur
- garder les commandes Tauri fines
- utiliser un format d'evenements stable
- imposer des budgets et timeouts
- auditer tous les appels outils

## 9. Backlog priorise

## P0
- fiabiliser CRUD teams
- definir roles
- implementer runtime equipe minimal
- brancher tool calling MCP sur un chemin heureux
- brancher PermissionManager sur shell/filesystem/MCP
- UI session equipe lisible
- tests e2e MVP

## P1
- parallelisme borne
- reviewer/critic
- resume final plus robuste
- reprise de session
- traces audit
- templates d'equipes

## P2
- remote/headless
- quotas
- scoring auto
- comparaison providers
- export sessions

## 10. Tests obligatoires

## 10.1 Unit
- create team
- add/remove member
- set role
- team session start
- event ordering
- permission gating
- tool routing

## 10.2 Integration
- create team puis session puis stream
- 2 agents sans outils
- 3 agents avec un outil MCP
- agent en echec puis synthese degradee
- permission refusee puis reprise

## 10.3 End-to-end

Scenario A:
- equipe research
- demande: resumer 3 sources
- output final + traces

Scenario B:
- equipe code
- demande: modifier fichier dans workspace
- approbation fichier
- execution MCP filesystem
- resultat final

Scenario C:
- equipe decision
- planner + critic + coordinator
- synthese finale argumentee

## 11. Definition of done

Vida UI pourra etre considere comme "vrai systeme d'equipes" quand:
- les equipes existent comme objets persistants stables
- les roles ont un effet runtime reel
- une session equipe orchestre plusieurs agents
- les agents peuvent appeler des outils sous controle
- les traces sont persistantes et lisibles
- les crashes sont geres
- les tests e2e passent
- un utilisateur peut s'en servir sans debuggage manuel

## 12. Proposition de sequence courte et rentable

Ordre recommande:

1. solidifier teams existants
2. definir roles + policy
3. livrer runtime equipe minimal
4. brancher filesystem + shell + MCP manager
5. brancher PermissionManager partout
6. finir UI session equipe
7. ajouter reprise + audit
8. seulement apres, parallelisme plus agressif

## 13. Estimation qualitative

Si tu veux un premier resultat vraiment utile:
- MVP equipe exploitable: effort moyen
- systeme equipe robuste et daily-driver: effort moyen a eleve

Mais cet effort est rentable parce que:
- l'architecture Vida est deja alignee avec le besoin
- contrairement a OpenFang, tu ne construis pas contre la base

## 14. Recommandation finale

Strategie conseillee:
- supprimer OpenFang comme base produit
- garder Vida UI comme base unique
- viser un MVP equipe propre avant toute extension exotique

Focus maximum:
- orchestration equipe
- permissions
- MCP/tool-calling
- stabilite session

Tout le reste vient apres.

