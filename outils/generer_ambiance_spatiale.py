from __future__ import annotations

import math
import struct
import wave
from pathlib import Path

TAUX_ECHANTILLONNAGE = 44_100
DUREE_SECONDES = 48.0
AMPLITUDE_MAX = 32_767
CHEMIN_SORTIE = (
    Path(__file__).resolve().parents[1] / "assets" / "sons" / "ambiance_spatiale.wav"
)


def quantifier_pour_boucle(frequence: float) -> float:
    return round(frequence * DUREE_SECONDES) / DUREE_SECONDES


def distance_boucle(temps: float, centre: float) -> float:
    distance = abs(temps - centre)
    return min(distance, DUREE_SECONDES - distance)


def poids_accord(temps: float, index: int, duree_segment: float) -> float:
    centre = (index + 0.5) * duree_segment
    distance = distance_boucle(temps, centre)
    if distance >= duree_segment:
        return 0.0
    progression = distance / duree_segment
    return math.cos((math.pi * 0.5) * progression) ** 4


def gain_stereo(panoramique: float) -> tuple[float, float]:
    panoramique = max(-1.0, min(1.0, panoramique))
    gauche = math.sqrt((1.0 - panoramique) * 0.5)
    droite = math.sqrt((1.0 + panoramique) * 0.5)
    return gauche, droite


def oscillateur_pad(frequence: float, temps: float, phase: float) -> float:
    fondamentale = math.sin(math.tau * frequence * temps + phase)
    harmonique_2 = 0.32 * math.sin(math.tau * frequence * 2.0 * temps + phase * 0.7)
    harmonique_3 = 0.12 * math.sin(math.tau * frequence * 3.0 * temps + phase * 1.3)
    respiration = 0.82 + 0.18 * math.sin(math.tau * (2.0 / DUREE_SECONDES) * temps + phase)
    return (fondamentale + harmonique_2 + harmonique_3) * respiration


def enveloppe_impulsion(temps_local: float, duree: float) -> float:
    if temps_local < 0.0 or temps_local > duree:
        return 0.0
    attaque = 0.16
    if temps_local < attaque:
        return math.sin((temps_local / attaque) * math.pi * 0.5) ** 2
    progression = (temps_local - attaque) / max(duree - attaque, 0.001)
    return math.cos(progression * math.pi * 0.5) ** 3


def oscillateur_impulsion(frequence: float, temps: float) -> float:
    fondamentale = math.sin(math.tau * frequence * temps)
    harmonique = 0.20 * math.sin(math.tau * frequence * 2.0 * temps)
    return fondamentale + harmonique


def generer() -> None:
    CHEMIN_SORTIE.parent.mkdir(parents=True, exist_ok=True)

    accords = [
        [110.00, 164.81, 220.00, 246.94, 261.63],
        [87.31, 130.81, 174.61, 220.00, 329.63],
        [130.81, 196.00, 261.63, 293.66, 329.63],
        [98.00, 146.83, 196.00, 220.00, 293.66],
    ]
    amplitudes_notes = [0.050, 0.035, 0.045, 0.024, 0.030]
    duree_segment = DUREE_SECONDES / len(accords)
    periode_impulsion = 2.4
    duree_impulsion = 1.7
    nombre_echantillons = int(TAUX_ECHANTILLONNAGE * DUREE_SECONDES)

    with wave.open(str(CHEMIN_SORTIE), "wb") as sortie:
        sortie.setnchannels(2)
        sortie.setsampwidth(2)
        sortie.setframerate(TAUX_ECHANTILLONNAGE)

        for index_echantillon in range(nombre_echantillons):
            temps = index_echantillon / TAUX_ECHANTILLONNAGE
            gauche = 0.0
            droite = 0.0

            for index_accord, accord in enumerate(accords):
                poids = poids_accord(temps, index_accord, duree_segment)
                if poids <= 0.0:
                    continue

                for index_note, note in enumerate(accord):
                    frequence = quantifier_pour_boucle(note)
                    phase = 0.8 * index_note + 0.35 * index_accord
                    panoramique = -0.34 + index_note * 0.17 + index_accord * 0.03
                    gain_gauche, gain_droite = gain_stereo(panoramique)
                    echantillon = (
                        amplitudes_notes[index_note]
                        * poids
                        * oscillateur_pad(frequence, temps, phase)
                    )
                    gauche += echantillon * gain_gauche
                    droite += echantillon * gain_droite

            temps_impulsion = temps % periode_impulsion
            accord_impulsion = int((temps % DUREE_SECONDES) / duree_segment) % len(accords)
            note_impulsion = quantifier_pour_boucle(accords[accord_impulsion][2] * 2.0)
            enveloppe = enveloppe_impulsion(temps_impulsion, duree_impulsion)
            if enveloppe > 0.0:
                impulsion = 0.050 * enveloppe * oscillateur_impulsion(note_impulsion, temps)
                gain_gauche, gain_droite = gain_stereo(0.08)
                gauche += impulsion * gain_gauche
                droite += impulsion * gain_droite

            basse = 0.030 * math.sin(math.tau * quantifier_pour_boucle(55.0) * temps)
            gauche += basse * 0.92
            droite += basse * 0.88

            souffle = 0.010 * math.sin(math.tau * quantifier_pour_boucle(880.0) * temps)
            souffle *= 0.5 + 0.5 * math.sin(math.tau * (1.0 / DUREE_SECONDES) * temps)
            gauche += souffle * 0.55
            droite += souffle * 0.45

            gauche = max(-0.95, min(0.95, gauche))
            droite = max(-0.95, min(0.95, droite))

            sortie.writeframesraw(
                struct.pack(
                    "<hh",
                    int(gauche * AMPLITUDE_MAX),
                    int(droite * AMPLITUDE_MAX),
                )
            )


if __name__ == "__main__":
    generer()
