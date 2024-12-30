import pickle
import array

class Entity:
    def __init__(self, position=None, attributes=None):
        """Initializes an entity with attributes."""

        self.position = array.array('f', position if position is not None else [0.0, 0.0, 0.0])
        self.orientation = array.array('f', position if position is not None else [1.0, 0.0])
        self.attributes = attributes or {}

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
