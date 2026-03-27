## Liste des trames complètes (commandes Modbus-RTU). 
   adresse esclave 0x06 (au lieu de 0x03 par défaut)

# 1. Commandes de lecture (fonction 03H)

Commande
Trame envoyée par le maître
Trame de réponse attendue (exemple)
Explication
Lecture des tensions A/B/C de la source I (registres 0x0006 à 0x0008)
06 03 00 06 00 03 E4 7D
06 03 06 00 DC 00 E6 00 F0 37 25
Uan=220 V, Ubn=230 V, Ucn=240 V

Lecture de la fréquence (registre 0x000D)
06 03 00 0D 00 01 14 7E
06 03 02 32 00 18 E4
Fréquence source I = 50 Hz, source II = 0 Hz

Lecture de l’état des sources (registre 0x004F)
06 03 00 4F 00 01 B4 6A
06 03 02 00 15 CC 4B
Exemple donné dans le manuel

Lecture de l’état de l’interrupteur (registre 0x0050)
06 03 00 50 00 01 85 AC
06 03 02 00 11 CD 88
Exemple donné dans le manuel

# 2. Commandes d’écriture (fonction 06H) – Contrôle
      Pour toutes ces commandes, la réponse est identique à la trame envoyée (écho).

Commande
Trame envoyée (et réponse)
Valeur écrite
Registre
Remarque
Exemple écriture : modifier la valeur de sous-tension source I
06 06 20 65 00 A0 93 DA
0x00A0 (160 V)
0x2065
Exemple du manuel (NXZ(H)MN/NZ5(H)M)

Restaurer les paramètres par défaut (adresse, baudrate, etc.)
06 06 28 00 00 02 00 1C
0x0002
0x2800
Contrôle commande – bit paramètre

Effacer l’historique (nombre de commutations, tensions max)
06 06 28 00 00 01 40 1D
0x0001
0x2800
Contrôle commande – bit historique

Entrer en mode contrôle distant
06 06 28 00 00 04 80 1E
0x0004
0x2800
Obligatoire avant toute commande de forçage

Forcer la position double séparation (double split)
06 06 27 00 00 FF C2 89
0x00FF
0x2700

Valide uniquement en contrôle distant
Forcer la position source I
06 06 27 00 00 00 82 C9
0x0000
0x2700

Valide uniquement en contrôle distant
Forcer la position source II
06 06 27 00 00 AA 02 B6
0x00AA
0x2700

Valide uniquement en contrôle distant
Sortir du mode contrôle distant
06 06 28 00 00 00 81 DD
0x0000
0x2800
Retour au mode normal

# 3. Exemple de trame d’erreur (fonction 83H)
Type
Trame envoyée par le maître
Trame de réponse attendue
Explication
Lecture d’un registre inexistant
06 03 A8 00 00 01 A5 DD
06 83 02 71 30

Code d’erreur 02 = registre invalide
# Remarques importantes:
	•	Avant toute commande de forçage (0x2700), il faut d’abord entrer en contrôle distant (0x2800 = 0x0004).
	•	Pour les séries NXZ(H)MN/NZ5(H)M, le verrouillage distant doit être désactivé sur le panneau.
	•	Les trames doivent être séparées d’au moins 200 ms.
	•	Tous les registres du tableau 4.1 du manuel sont accessibles en lecture (03H) ; 
    les registres R/W sont accessibles en écriture (06H) avec la même structure que les exemples ci-dessus.
	•	CRC recalculé avec l’algorithme officiel du document (polynôme 0xA001, init 0xFFFF).

