import pickle
import array

class Entity:
    def __init__(self, position=None, orientation=None, attributes=None, level=1):
        """Initializes an entity with attributes."""

        self.position = array('f', position if position is not None else [0.0, 0.0, 0.0])
        self.orientation = array('f', position if position is not None else [1.0, 0.0])
        # self.position = array('f', [1.0, 2.0, 3.0])

        self.attributes = attributes or {}
        self.level = level

        self.id = 0
        self.manager_id = 0

        self.type = EntityType.NPC

    def update_attribute(self, key, value):
        """Updates or adds an attribute."""
        self.attributes[key] = value

    def get_all_attributes(self):
        """Returns all attributes."""
        return self.attributes

    def serialize(self):
        """Serializes the Entity to a pickle string."""
        return pickle.dumps(self)

    @staticmethod
    def deserialize(data):
        """Deserializes an Entity from a pickle string."""
        return pickle.loads(data)

    def debug(self):
        """Prints all attributes of the entity."""
        print(f"Position: {list(self.position)}")
        print(f"Orientation: {list(self.orientation)}")
        print("Entity attributes:")
        for key, value in self.attributes.items():
            print(f" - {key}: {value}")

    def update(self):
        """Update"""
        pass

    def event(self, event, value):
        """Event"""
        pass

    def user_event(self, event, value):
        """User event. Only send to EntityType.PLAYER"""
