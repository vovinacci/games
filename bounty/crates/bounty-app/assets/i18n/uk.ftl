# Bounty: текст на екрані, українська. Кожен ключ має бути і в en-GB.ftl (це перевіряє тест).

title = Bounty
prototype-note = Пробні правила, не правила оригінальної гри
modern-note = Змінені правила: відтворені, з власними змінами
reference-note = Правила з іншого відтворення оригіналу, ще не звірені з ним
header-battle = Бій: { $id }, зерно { $seed }
round = Раунд { $n }

# Назви. Загін завжди названо разом зі стороною і в одній формі, щоб речення не відмінювали назву.
troop-warden = Вартові
troop-slinger = Пращники
troop-skyrider = Небесні вершники
troop-sprites = Феї
troop-wolves = Вовки
troop-archers = Лучники
name-player = ваші { $troop }
name-enemy = ворожі { $troop }
spell-blast = Вибух
spell-mend = Зцілення

# Бічна панель
turn-of = Хід: { $name }
enemy-turn = Ходить ворог
stat-count = Істот: { $n }
stat-health = Здоров'я кожної: { $hp }, поранення: { $injury }
stat-movement = Залишок руху: { $n }
stat-shots = Залишок пострілів: { $n }
stat-flights = Залишок польотів: { $n }
stat-leadership = Лідерство героя: { $n }
stat-fighting = Б'ються: { $n } з { $count }, на скільки вистачає лідерства
stat-morale = Бойовий дух: { $morale }
morale-because = Знижують: { $names }
morale-low = низький, шкода вдвічі менша
morale-normal = звичайний
morale-high = високий, шкода в півтора раза більша
out-of-control = Загін не слухається: б'ється з усіма і ходить сам
actions = Можливі дії
key-move = Стрілки: рухати курсор
key-act = Enter: { $what }
act-melee = ударити, ціль: { $name }
act-shoot = стріляти, ціль: { $name }
act-walk = ступити сюди
act-fly = перелетіти сюди
act-none = тут нічого зробити не можна
key-wait = W: чекати до кінця раунду
key-pass = Пробіл: завершити хід
key-cast = { $key }: { $spell }, ціль під курсором (залишилось: { $n })
key-language = L: мова
key-again = R: новий бій
key-quit = Esc: вийти

# Журнал
log-round = Починається раунд { $n }
log-damage = Атака: { $attacker }, ціль: { $target }. Шкода { $damage } (кидок { $roll }, істот: { $attackers }{ $morale }), втрати: { $losses }
log-retaliation = Удар у відповідь: { $attacker }, ціль: { $target }. Шкода { $damage } (кидок { $roll }, істот: { $attackers }{ $morale }), втрати: { $losses }
log-morale-low = , низький бойовий дух
log-morale-high = , високий бойовий дух
log-control = Загін знову слухається: { $name }
log-spell-damage = { $spell }, ціль: { $target }. Шкода { $damage }, втрати: { $losses }
log-removed = Загін знищено: { $name }
log-moved = Перейшли: { $name }
log-flew = Перелетіли: { $name }
log-waited = Чекають: { $name }
log-healed = { $name }: зцілено поранень: { $removed }

outcome-player_win = Перемога
outcome-enemy_win = Поразка
outcome-budget_exceeded = Без результату: вичерпано ліміт раундів
