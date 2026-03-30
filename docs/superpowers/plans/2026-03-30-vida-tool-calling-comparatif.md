# Vida UI — Comparatif Réel entre l'Audit et `2026-03-30-vida-tool-calling.md`

**Date:** 2026-03-30  
**Document comparé:** `docs/superpowers/plans/2026-03-30-vida-tool-calling.md`  
**Référence audit:** `docs/superpowers/plans/2026-03-30-vida-audit-report.md`

## 1. Objet du Document

Ce document compare:

- ce que le plan `2026-03-30-vida-tool-calling.md` attend
- ce qui est réellement présent dans le code au moment de l'audit
- mon avis technique sur l'écart entre la cible et l'état réel

## 2. Synthèse Courte

Le plan tool-calling est bien structuré et techniquement cohérent. En revanche, son état réel dans le dépôt est **partiel**:

- **Task 1** est globalement engagée
- les tâches suivantes critiques ne sont pas livrées
- le produit reste dans un mode "préparation du tool-calling" et non "tool-calling fiable opérationnel"

Mon avis global:

- le plan est **bon**
- son exécution est **incomplète**
- le risque principal est de surestimer l'avancement réel parce que certaines structures ont déjà été ajoutées

## 3. Comparatif Point par Point

### 3.1 Ce que le plan demande

Le plan attend explicitement:

```text
crates/vida-core/src/
├── agent_loop.rs
├── tool_validator.rs
├── engine.rs                 # Add agent_chat() that uses AgentLoop

src/hooks/
├── useAgentStream.ts
src/components/chat/
├── ToolCallBubble.tsx
```

Le plan attend aussi:

- support tools dans tous les providers
- validation JSON Schema avant exécution
- boucle LLM ↔ MCP complète
- streaming d'événements tool call vers le frontend

### 3.2 Ce qui est réellement présent

#### Livré / présent

- `ToolDefinition` ajouté dans `crates/vida-providers/src/traits.rs`
- `ToolCall` ajouté dans `crates/vida-providers/src/traits.rs`
- `ChatRole::Tool` ajouté
- `CompletionOptions.tools` présent
- `CompletionResponse.tool_calls` présent
- `ChatMessage.tool_call_id` et `ChatMessage.name` présents
- adaptation partielle des providers à ces types

#### Non livré / absent

- `crates/vida-core/src/agent_loop.rs`
- `crates/vida-core/src/tool_validator.rs`
- méthode `agent_chat()` dédiée dans le moteur
- `src/hooks/useAgentStream.ts`
- `src/components/chat/ToolCallBubble.tsx`

#### Détail important

Tous les providers continuent à renvoyer:

```rust
tool_calls: vec![]
```

Donc:

- le trait est prêt
- la donnée réelle ne l'est pas

### 3.3 Tableau de comparaison

| Élément du plan | Attendu | Réel | État |
|---|---|---|---|
| Types outils dans `traits.rs` | Oui | Oui | partiellement livré |
| Tests sérialisation tools | Oui | Oui | livré |
| OpenAI tool calling | Oui | Non | non livré |
| Anthropic tool use | Oui | Non | non livré |
| Gemini function calling | Oui | Non | non livré |
| Ollama native tool calling | Oui | Non | non livré |
| `agent_loop.rs` | Oui | Non | non livré |
| `tool_validator.rs` | Oui | Non | non livré |
| validation JSON Schema | Oui | Non | non livré |
| orchestration LLM ↔ MCP | Oui | Non | non livré |
| `agent_stream_completion` | Oui | Non identifié comme livré | non livré |
| `useAgentStream.ts` | Oui | Non | non livré |
| `ToolCallBubble.tsx` | Oui | Non | non livré |
| UI tool-calls native | Oui | Non, parsing texte actuel | partiel |

## 4. Écarts Techniques Réels

### 4.1 Le plus trompeur: la couche type est prête

Le dépôt contient déjà:

- les bons types Rust
- des champs `tools`
- des champs `tool_calls`

Cela peut donner l'impression que le support tool-calling est avancé. En réalité, il manque encore la partie déterminante:

- construction de la requête provider avec tools
- parsing de la réponse provider en tool calls réels
- validation arguments
- exécution MCP
- boucle de continuation

En d'autres termes:

- **l'API interne est préparée**
- **la fonctionnalité produit n'est pas livrée**

### 4.2 L'UI actuelle confirme cet écart

L'UI ne consomme pas un flux structuré de tool-calls natifs. Elle parse encore du texte avec:

- `<tool_call>...</tool_call>`
- `<tool_result>...</tool_result>`

C'est une stratégie de transition, pas un état final "fiable" au sens du plan.

### 4.3 Le MCP présent ne suffit pas

Le fait que `McpManager` existe ne signifie pas que le plan tool-calling est réalisé.

Le MCP actuel sait:

- démarrer un serveur
- lister des tools
- appeler un tool

Mais le plan exige en plus:

- qu'un LLM demande un tool de façon structurée
- que l'appel soit validé
- que le résultat soit réinjecté
- que la boucle continue jusqu'à réponse finale

Cette orchestration n'est pas présente.

## 5. Mon Avis Technique

### 5.1 Sur le plan lui-même

Le plan `2026-03-30-vida-tool-calling.md` est bon:

- il découpe correctement les couches
- il cible les bons fichiers
- il impose une validation JSON Schema avant exécution
- il prend en compte le streaming frontend
- il sépare bien trait provider, orchestration moteur et UI

Je n'ai pas d'objection majeure sur son architecture cible.

### 5.2 Sur l'état réel d'avancement

Mon avis est net:

- **la Task 1 est partiellement à majoritairement faite**
- **les tâches 2 et suivantes ne sont pas réellement livrées**
- **le projet ne doit pas être présenté comme disposant d'un tool-calling fiable**

Le principal danger est un faux sentiment d'avancement.

### 5.3 Sur la priorité de correction

Je recommande l'ordre suivant:

1. finir `tool_validator.rs`
2. implémenter `agent_loop.rs`
3. activer d'abord un provider unique de référence
   - recommandé: OpenAI-compatible ou Ollama
4. ajouter un faux MCP server de test pour l'end-to-end
5. seulement ensuite généraliser aux autres providers

La bonne stratégie n'est pas de "patcher partout à la fois", mais de livrer un chemin fonctionnel complet de bout en bout.

## 6. Ce qui est déjà récupérable

Bonne nouvelle: le travail déjà fait n'est pas à jeter.

Réutilisable immédiatement:

- les types du trait provider
- les champs optionnels dans `ChatMessage`
- `CompletionOptions.tools`
- `CompletionResponse.tool_calls`
- `McpManager`
- la base UI existante pour afficher des appels d'outils

Ce qui manque est surtout l'exécution orchestrée, pas la fondation des structures.

## 7. Conclusion

### Conclusion factuelle

Le comparatif réel montre:

- un **plan solide**
- une **implémentation commencée**
- une **fonctionnalité non finalisée**

### Mon avis final

Si l'objectif est de savoir si le dépôt actuel "a le tool-calling fiable", ma réponse est:

**non**

Si l'objectif est de savoir si le dépôt actuel "a déjà les bases suffisantes pour le terminer sans refonte", ma réponse est:

**oui**

La situation n'est donc pas "architecture ratée", mais "chantier central encore incomplet". C'est une différence importante:

- le cap technique est bon
- l'intégration finale reste à faire
