from array import array

class Monster(Entity):
    def __init__(self, position=None, orientation=None, attributes=None, level=1):
        """Initializes a Monster."""
        super().__init__(position, orientation, attributes)

    def attack(self, target):
        """Attacks another fighter, reducing its health."""
        if not isinstance(target, Monster):
            print("Target must be an EntityFighter.")
            return

        target.take_damage(self.damage)
        print(f"{self} attacked {target}, dealing {self.damage} damage!")

    def take_damage(self, amount):
        """Reduces the fighter's health."""
        self.health -= amount
        print(f"{self} took {amount} damage, health is now {self.health}")
        if self.health <= 0:
            print(f"{self} has been defeated!")

    def debug(self):
        """Prints the fighter's specific attributes along with inherited ones."""
        super().debug()  # Call the parent class's debug method
        print(f"Health: {self.health}")
        print(f"Damage: {self.damage}")

    def __str__(self):
        """String representation of the fighter."""
        return f"EntityFighter at {list(self.position)} with {self.health} HP and {self.damage} damage"