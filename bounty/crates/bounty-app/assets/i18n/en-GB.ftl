# Bounty: text on screen, British English. Every key must also be in uk.ftl (a test checks).

title = Bounty
prototype-note = Test rules, not those of the original game
modern-note = Variant rules: the remake's, with this game's own changes
reference-note = Rules from another remake of the original, not yet checked against it
header-battle = Battle: { $id }, seed { $seed }
round = Round { $n }

# Names. A stack is named with its side, always in the same form, so sentences never have to bend the name.
troop-warden = Wardens
troop-slinger = Slingers
troop-skyrider = Skyriders
troop-sprites = Sprites
troop-wolves = Wolves
troop-archers = Archers
name-player = your { $troop }
name-enemy = enemy { $troop }
spell-blast = Blast
spell-mend = Mend

# Sidebar
turn-of = Turn: { $name }
enemy-turn = The enemy is moving
stat-count = Creatures: { $n }
stat-health = Health of each: { $hp }, injury: { $injury }
stat-movement = Movement left: { $n }
stat-shots = Shots left: { $n }
stat-flights = Flights left: { $n }
stat-leadership = Hero's leadership: { $n }
stat-fighting = Fighting: { $n } of { $count }, as far as the leadership reaches
stat-morale = Morale: { $morale }
morale-because = Lowered by: { $names }
morale-low = low, damage halved
morale-normal = normal
morale-high = high, damage and half again
out-of-control = Out of control: it fights everyone and moves by itself
actions = What you can do
key-move = Arrows: move the cursor
key-act = Enter: { $what }
act-melee = strike { $name }
act-shoot = shoot at { $name }
act-walk = step here
act-fly = fly here
act-none = nothing to do here
key-wait = W: wait until the end of the round
key-pass = Space: end the turn
key-cast = { $key }: { $spell } on the stack under the cursor ({ $n } left)
key-language = L: language
key-again = R: fight again
key-quit = Esc: quit

# Log
log-round = Round { $n } begins
log-damage = Attack: { $attacker } on { $target }. Damage { $damage } (roll { $roll }, creatures: { $attackers }{ $morale }), lost: { $losses }
log-retaliation = Strike back: { $attacker } on { $target }. Damage { $damage } (roll { $roll }, creatures: { $attackers }{ $morale }), lost: { $losses }
log-morale-low = , low morale
log-morale-high = , high morale
log-control = Back under control: { $name }
log-spell-damage = { $spell } on { $target }. Damage { $damage }, lost: { $losses }
log-removed = Destroyed: { $name }
log-moved = Moved: { $name }
log-flew = Flew: { $name }
log-waited = Waiting: { $name }
log-healed = { $name }: injury healed, { $removed }

outcome-player_win = Victory
outcome-enemy_win = Defeat
outcome-budget_exceeded = No result: the round limit was reached
