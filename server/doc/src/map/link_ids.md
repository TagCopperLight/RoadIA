# Link IDs & how to obtain them

Ce chapitre explique comment récupérer les identifiants de liens internes et pourquoi ils sont indispensables pour les feux et les mouvements de carrefour.

## Principe

`build_intersections(&mut map)` génère les `InternalLane` et les `Link` de chaque carrefour. C'est à ce moment que les identifiants de lien sont attribués et qu'ils deviennent utilisables par le moteur et par les contrôleurs de feux.

## Workflow conseillé

1. Finaliser complètement la topologie de la carte.
2. Appeler `build_intersections(&mut map)`.
3. Inspecter la structure de la carte côté serveur pour relever les identifiants de `InternalLane` et de `Link` utiles.
4. Construire les phases de feux ou les priorités de rond-point à partir de ces identifiants.

## Ce qu'il faut observer

- Les identifiants de liens sont stables tant que la topologie ne change pas.
- Toute modification structurelle de la carte peut entraîner une régénération des identifiants.
- Les paquets de sérialisation grand public n'exposent pas directement ces identifiants, donc l'inspection doit se faire côté serveur.

## Usage pratique

Les identifiants de liens servent principalement à:

- configurer des phases de feux tricolores;
- vérifier les conflits dans les intersections;
- comprendre quel mouvement le moteur a prévu dans le `DrivePlan` d'un véhicule.
