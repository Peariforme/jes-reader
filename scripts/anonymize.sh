#!/bin/sh
# Filtre « clean » Git : remplace les userids IBM Z Xplore (Z suivi de
# exactement 5 chiffres) par Z99999 dans les fichiers du corpus, au
# moment du `git add`. La copie de travail garde les vraies valeurs.
#
# Le remplacement est de même longueur : les colonnes des sysouts ne
# bougent pas. Lecture et écriture en binaire : les CRLF sont conservés
# (le sed de Git for Windows les supprime, d'où perl).
#
# Lit stdin, écrit stdout. Activation : voir README, section Développement.
exec perl -0777 -pe 'BEGIN { binmode STDIN; binmode STDOUT } s/(?<![A-Za-z0-9])Z[0-9]{5}(?![0-9])/Z99999/g'
